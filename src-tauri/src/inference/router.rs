// ─── Model Router ───
//
// Section 6.5 of the PRD.
// Deterministic routing: maps task types + agent roles to the correct model.
// No dynamic or heuristic routing.

use super::ModelId;

/// Logical task classification that drives model selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelRoute {
    /// Deep analysis — Qwen3-14B, thinking enabled
    DeepReasoning,
    /// Fast, low-latency responses — Qwen3-8B, thinking disabled
    FastInteraction,
    /// Embedding generation — nomic-embed-text
    Embedding,
}

impl ModelRoute {
    /// Get the ModelId for this route.
    pub fn model_id(&self) -> ModelId {
        match self {
            ModelRoute::DeepReasoning => ModelId::Primary,
            ModelRoute::FastInteraction => ModelId::Fast,
            ModelRoute::Embedding => ModelId::Embedding,
        }
    }

    /// Whether thinking mode should be enabled for this route.
    pub fn enable_thinking(&self) -> bool {
        matches!(self, ModelRoute::DeepReasoning)
    }

    /// Typical temperature for this route.
    pub fn default_temperature(&self) -> f64 {
        match self {
            ModelRoute::DeepReasoning => 0.3, // lower temp for factual accuracy
            ModelRoute::FastInteraction => 0.7,
            ModelRoute::Embedding => 0.0, // not used for embeddings
        }
    }

    /// Typical max tokens for this route.
    pub fn default_max_tokens(&self) -> u32 {
        match self {
            ModelRoute::DeepReasoning => 4096,
            ModelRoute::FastInteraction => 1024,
            ModelRoute::Embedding => 0,
        }
    }
}

/// Route a task to the correct model.
///
/// This is the deterministic routing function from PRD Section 6.5.
/// Every task type is assigned to exactly one model.
pub fn route_model(task: InferenceTask) -> ModelRoute {
    match task {
        InferenceTask::PostGameAnalysis
        | InferenceTask::OpeningInstruction
        | InferenceTask::EndgameInstruction
        | InferenceTask::StrategicExplanation
        | InferenceTask::ComplexCoaching => ModelRoute::DeepReasoning,

        InferenceTask::LiveCoachingNote
        | InferenceTask::ShortConversationalReply
        | InferenceTask::CurriculumGeneration
        | InferenceTask::OrchestrationSubTask => ModelRoute::FastInteraction,

        InferenceTask::EmbedText => ModelRoute::Embedding,
    }
}

/// All task types the system can route (PRD Section 6.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferenceTask {
    // ─── Qwen3-14B, thinking enabled ───
    PostGameAnalysis,
    OpeningInstruction,
    EndgameInstruction,
    StrategicExplanation,
    ComplexCoaching,

    // ─── Qwen3-8B, thinking disabled ───
    LiveCoachingNote,
    ShortConversationalReply,
    CurriculumGeneration,
    OrchestrationSubTask,

    // ─── Embedding model ───
    EmbedText,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deep_reasoning_routes() {
        assert_eq!(
            route_model(InferenceTask::PostGameAnalysis).model_id(),
            ModelId::Primary
        );
        assert!(route_model(InferenceTask::PostGameAnalysis).enable_thinking());
        assert_eq!(
            route_model(InferenceTask::ComplexCoaching).model_id(),
            ModelId::Primary
        );
    }

    #[test]
    fn test_fast_interaction_routes() {
        assert_eq!(
            route_model(InferenceTask::LiveCoachingNote).model_id(),
            ModelId::Fast
        );
        assert!(!route_model(InferenceTask::LiveCoachingNote).enable_thinking());
        assert_eq!(
            route_model(InferenceTask::ShortConversationalReply).model_id(),
            ModelId::Fast
        );
    }

    #[test]
    fn test_embedding_route() {
        assert_eq!(
            route_model(InferenceTask::EmbedText).model_id(),
            ModelId::Embedding
        );
    }
}
