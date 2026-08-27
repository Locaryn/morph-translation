//! Locaryn Machine Translation Plugin
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationRequest {
    pub text: String,
    pub source_lang: Option<String>,
    pub target_lang: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationResult {
    pub translated_text: String,
    pub detected_source_lang: String,
    pub target_lang: String,
}

/// Non implemente. La signature est conservee pour que l'interface et le
/// serveur MCP gardent leur forme, mais l'appel echoue franchement plutot
/// que de fabriquer un resultat.
pub async fn translate_text(_req: TranslationRequest) -> Result<TranslationResult, String> {
    Err("La traduction n'est pas implementee : ce morph ne traduit rien et renvoyait auparavant le texte d'origine.".into())
}
