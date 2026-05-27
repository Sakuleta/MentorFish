// ─── Pipeline Engine ───
//
// Executes agent chains in order per pipeline type.
// Manages pre-fetching, error handling, and model selection.

use crate::agents::curriculum::StudyPlan;
use crate::agents::executor;
use crate::agents::memory::ProfileDelta;
use crate::agents::pedagogical::{ExplanationDepth, FinalExplanation};
use crate::agents::strategic::StrategicSummary;
use crate::agents::tactical::TacticalSummary;
use crate::agents::theory::TheoryOutput;
use crate::inference::InferenceClient;
use crate::orchestrator::{OrchestratorContext, PipelineCallbacks, PipelineError, PipelineType};

/// Result of a completed pipeline execution.
#[derive(Debug)]
pub struct PipelineResult {
    pub tactical_summary: Option<TacticalSummary>,
    pub strategic_summary: Option<StrategicSummary>,
    pub theory_output: Option<TheoryOutput>,
    pub profile_delta: Option<ProfileDelta>,
    pub study_plan: Option<StudyPlan>,
    pub explanation: FinalExplanation,
    pub errors: Vec<PipelineError>,
    /// PGN from the context, preserved for database persistence.
    pub pgn: Option<String>,
    /// Game result, preserved for database persistence.
    pub game_result: Option<String>,
}

/// Execute a pipeline from a pre-populated OrchestratorContext.
///
/// This is the main entry point for all pipeline types.
/// It runs the agent chain in order, respecting the error handling policy:
/// if an agent fails, log the error and continue with remaining agents.
///
/// If `callbacks` are provided, progress is reported to the frontend
/// in real time (engine-progress, coaching-alert, streaming-token events).
pub async fn execute(
    client: &dyn InferenceClient,
    ctx: &OrchestratorContext,
    callbacks: PipelineCallbacks,
) -> PipelineResult {
    let mut errors = Vec::new();
    let mut tactical: Option<TacticalSummary> = None;
    let mut strategic: Option<StrategicSummary> = None;
    let mut theory: Option<TheoryOutput> = None;
    let mut profile_delta: Option<ProfileDelta> = None;
    let mut study_plan: Option<StudyPlan> = None;

    let chain = crate::orchestrator::agent_chain(ctx.pipeline_type);

    for agent_name in &chain {
        match *agent_name {
            "tactical" => {
                tactical = Some(executor::run_tactical(&ctx.engine_output, &ctx.features));
                if let Some(ref cb) = callbacks.on_agent_complete {
                    cb("tactical", "Tactical analysis complete");
                }
            }
            "strategic" => {
                match executor::run_strategic(
                    client,
                    &ctx.features,
                    tactical.as_ref().unwrap_or(&TacticalSummary::default()),
                    &ctx.rag_results,
                )
                .await
                {
                    Ok(s) => {
                        if let Some(ref cb) = callbacks.on_agent_complete {
                            cb("strategic", "Strategic analysis complete");
                        }
                        strategic = Some(s);
                    }
                    Err(e) => errors.push(PipelineError::AgentError {
                        agent: "strategic".into(),
                        message: e.to_string(),
                    }),
                }
            }
            "theory" => match executor::run_theory(client, &ctx.position, &ctx.rag_results).await {
                Ok(t) => {
                    if let Some(ref cb) = callbacks.on_agent_complete {
                        cb("theory", "Opening theory check complete");
                    }
                    theory = Some(t);
                }
                Err(e) => errors.push(PipelineError::AgentError {
                    agent: "theory".into(),
                    message: e.to_string(),
                }),
            },
            "memory" => {
                profile_delta = Some(executor::run_memory(
                    tactical.as_ref().unwrap_or(&TacticalSummary::default()),
                    strategic.as_ref().unwrap_or(&StrategicSummary::default()),
                ));
                if let Some(ref cb) = callbacks.on_agent_complete {
                    cb("memory", "Player profile updated");
                }
            }
            "curriculum" => match executor::run_curriculum(client, &ctx.user_profile).await {
                Ok(p) => {
                    if let Some(ref cb) = callbacks.on_agent_complete {
                        cb("curriculum", "Study plan generated");
                    }
                    study_plan = Some(p);
                }
                Err(e) => errors.push(PipelineError::AgentError {
                    agent: "curriculum".into(),
                    message: e.to_string(),
                }),
            },
            "pedagogical" => {
                // ── Conversational pipeline: multi-turn chat with history ──
                if ctx.pipeline_type == PipelineType::Conversational {
                    match executor::run_conversational_chat(
                        client,
                        tactical.as_ref(),
                        strategic.as_ref(),
                        &ctx.persona,
                        &ctx.conversation_history,
                        callbacks.on_streaming_token.clone(),
                    )
                    .await
                    {
                        Ok(reply) => {
                            return PipelineResult {
                                tactical_summary: tactical,
                                strategic_summary: strategic,
                                theory_output: theory,
                                profile_delta,
                                study_plan,
                                explanation: FinalExplanation {
                                    text: reply,
                                    layer_breakdown: vec![],
                                    confidence: 0.85,
                                    low_confidence_note: None,
                                },
                                errors,
                                pgn: ctx.pgn.clone(),
                                game_result: ctx.game_result.clone(),
                            };
                        }
                        Err(e) => {
                            errors.push(PipelineError::AgentError {
                                agent: "conversational".into(),
                                message: e.to_string(),
                            });
                            return PipelineResult {
                                tactical_summary: tactical,
                                strategic_summary: strategic,
                                theory_output: theory,
                                profile_delta,
                                study_plan,
                                explanation: FinalExplanation {
                                    text:
                                        "I'm having trouble responding right now. Please try again."
                                            .into(),
                                    layer_breakdown: vec![],
                                    confidence: 0.0,
                                    low_confidence_note: Some(format!(
                                        "Conversational error: {}",
                                        errors
                                            .last()
                                            .map(|e| format!("{:?}", e))
                                            .unwrap_or_default()
                                    )),
                                },
                                errors,
                                pgn: ctx.pgn.clone(),
                                game_result: ctx.game_result.clone(),
                            };
                        }
                    }
                }

                // ── Standard pedagogical: structured explanation with layers ──
                let depth = match ctx.pipeline_type {
                    PipelineType::PostGame => ExplanationDepth::Full,
                    PipelineType::LiveCoaching => ExplanationDepth::Brief,
                    PipelineType::Theory => ExplanationDepth::Standard,
                    PipelineType::Curriculum => ExplanationDepth::Standard,
                    PipelineType::Conversational => ExplanationDepth::Standard, // unreachable, handled above
                };

                let mut confidence_flags = Vec::new();
                if ctx.rag_results.chunks.is_empty() {
                    errors.push(PipelineError::EmptyRagResults);
                    confidence_flags.push("low_confidence".into());
                }

                match executor::run_pedagogical(
                    client,
                    tactical.as_ref(),
                    strategic.as_ref(),
                    theory.as_ref(),
                    &ctx.user_profile,
                    &ctx.persona,
                    depth,
                    &confidence_flags,
                    callbacks.on_streaming_token.clone(),
                )
                .await
                {
                    Ok(explanation) => {
                        return PipelineResult {
                            tactical_summary: tactical,
                            strategic_summary: strategic,
                            theory_output: theory,
                            profile_delta,
                            study_plan,
                            explanation,
                            errors,
                            pgn: ctx.pgn.clone(),
                            game_result: ctx.game_result.clone(),
                        };
                    }
                    Err(e) => {
                        errors.push(PipelineError::AgentError {
                            agent: "pedagogical".into(),
                            message: e.to_string(),
                        });
                        // Return fallback explanation
                        return PipelineResult {
                            tactical_summary: tactical,
                            strategic_summary: strategic,
                            theory_output: theory,
                            profile_delta,
                            study_plan,
                            explanation: FinalExplanation {
                                text: "Analysis pipeline encountered an error. Please try again."
                                    .into(),
                                layer_breakdown: vec![],
                                confidence: 0.0,
                                low_confidence_note: Some(format!(
                                    "Pipeline error: {}",
                                    errors
                                        .last()
                                        .map(|e| format!("{:?}", e))
                                        .unwrap_or_default()
                                )),
                            },
                            errors,
                            pgn: ctx.pgn.clone(),
                            game_result: ctx.game_result.clone(),
                        };
                    }
                }
            }
            _ => {}
        }
    }

    // Should not reach here — pedagogical agent always returns
    PipelineResult {
        tactical_summary: tactical,
        strategic_summary: strategic,
        theory_output: theory,
        profile_delta,
        study_plan,
        explanation: FinalExplanation {
            text: "Pipeline completed without pedagogical agent.".into(),
            layer_breakdown: vec![],
            confidence: 0.5,
            low_confidence_note: None,
        },
        errors,
        pgn: None,
        game_result: None,
    }
}

// ─── Default implementations for fallback ───

impl Default for TacticalSummary {
    fn default() -> Self {
        Self {
            blunders: vec![],
            missed_tactics: vec![],
            eval_swings: vec![],
            forcing_sequences: vec![],
            confidence: 0.5,
        }
    }
}

impl Default for StrategicSummary {
    fn default() -> Self {
        Self {
            imbalances: vec![],
            plans: vec![],
            pawn_structure: crate::agents::strategic::PawnStructureClassification {
                structure_type: "unknown".into(),
                plans: vec![],
            },
            key_weaknesses: vec![],
            positional_themes: vec![],
            confidence: 0.5,
        }
    }
}
