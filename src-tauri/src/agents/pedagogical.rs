// ─── Pedagogical Agent ───
//
// Section 7.4, Agent C (PRD) and Section 15.
// The sole explanation assembler. All upstream outputs converge here.

use crate::ConfidenceScore;
use serde::{Deserialize, Serialize};
use specta::Type;

/// The final assembled explanation, surfaced in the UI.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct FinalExplanation {
    pub text: String,
    pub layer_breakdown: Vec<LayerContent>,
    pub confidence: ConfidenceScore,
    pub low_confidence_note: Option<String>,
}

/// One layer in the hierarchical explanation (Section 2.5).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LayerContent {
    pub layer: u32,
    pub layer_name: String,
    pub content: String,
    pub confidence: ConfidenceScore,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
pub enum ExplanationDepth {
    Brief,
    Standard,
    Full,
}

/// Coaching persona (Section 14).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Type)]
pub enum Persona {
    SovietCoach,
    #[default]
    ModernGM,
    CalmTeacher,
    BrutalAnalyst,
    PsychologicalMentor,
}

impl Persona {
    /// Returns the system prompt fragment for this persona.
    pub fn system_prompt(&self) -> &str {
        match self {
            Persona::SovietCoach => {
                "You are a demanding Soviet chess coach. Be direct, dense, and technical. \
                 State criticisms as facts. Use maximum depth. Reference classical games."
            }
            Persona::ModernGM => {
                "You are a pragmatic, efficient chess grandmaster coach. Balance technical \
                 precision with accessible language. Be practical and balanced."
            }
            Persona::CalmTeacher => {
                "You are a patient, encouraging chess teacher. Use clear, accessible language. \
                 Frame criticism constructively as learning opportunities. Adapt depth to the student."
            }
            Persona::BrutalAnalyst => {
                "You are a cold, precise chess analyst. Use purely technical language with no \
                 softening. Provide maximum depth and no sugar-coating."
            }
            Persona::PsychologicalMentor => {
                "You are a reflective, process-focused chess mentor. Use plain, empathetic language. \
                 Connect positions to recurring patterns. Add metacognitive insight."
            }
        }
    }
}

// ─── Explanation Assembler ───

/// Placeholder — real implementation calls the LLM via InferenceClient.
pub struct PedagogicalInput<'a> {
    pub tactical_summary: Option<&'a super::tactical::TacticalSummary>,
    pub strategic_summary: Option<&'a super::strategic::StrategicSummary>,
    pub theory_output: Option<&'a super::theory::TheoryOutput>,
    pub curriculum_output: Option<&'a super::curriculum::StudyPlan>,
    pub user_profile: &'a super::UserProfile,
    pub persona: Persona,
    pub depth: ExplanationDepth,
    pub confidence_flags: Vec<String>,
}

/// Assembles the final explanation. Calls LLM in production; returns stub for now.
pub fn assemble_stub(input: PedagogicalInput) -> FinalExplanation {
    let persona_name = format!("{:?}", input.persona);
    let depth_name = format!("{:?}", input.depth);

    let layers = vec![
        LayerContent {
            layer: 1,
            layer_name: "Move Truth".to_string(),
            content: "Engine evaluation and best move (stub).".to_string(),
            confidence: 1.0,
        },
        LayerContent {
            layer: 2,
            layer_name: "Tactical Logic".to_string(),
            content: "Concrete tactical mechanism (stub).".to_string(),
            confidence: 0.8,
        },
        LayerContent {
            layer: 3,
            layer_name: "Strategic Meaning".to_string(),
            content: "Positional principle at work (stub).".to_string(),
            confidence: 0.6,
        },
    ];

    let note = if input
        .confidence_flags
        .contains(&"low_confidence".to_string())
    {
        Some(
            "Note: Limited reference material was found for this position type. \
             The explanation is based on general principles."
                .to_string(),
        )
    } else {
        None
    };

    FinalExplanation {
        text: format!(
            "[{}] Analysis at depth {} (stub — LLM integration pending)",
            persona_name, depth_name
        ),
        layer_breakdown: layers,
        confidence: 0.7,
        low_confidence_note: note,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persona_system_prompts() {
        let personas = [
            Persona::SovietCoach,
            Persona::ModernGM,
            Persona::CalmTeacher,
            Persona::BrutalAnalyst,
            Persona::PsychologicalMentor,
        ];
        for persona in &personas {
            let prompt = persona.system_prompt();
            assert!(
                !prompt.is_empty(),
                "Persona {:?} returned empty system prompt",
                persona
            );
        }
    }

    #[test]
    fn test_persona_default() {
        assert_eq!(Persona::default(), Persona::ModernGM);
    }

    #[test]
    fn test_explanation_depth_variants() {
        // Verify all three variants exist and are distinct
        let brief = ExplanationDepth::Brief;
        let standard = ExplanationDepth::Standard;
        let full = ExplanationDepth::Full;

        // They should be distinct (not all equal)
        let variants = [
            format!("{:?}", brief),
            format!("{:?}", standard),
            format!("{:?}", full),
        ];
        // All debug representations should be different
        assert_ne!(variants[0], variants[1]);
        assert_ne!(variants[1], variants[2]);
        assert_ne!(variants[0], variants[2]);
    }
}
