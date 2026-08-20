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

pub async fn translate_text(req: TranslationRequest) -> Result<TranslationResult, String> {
    if req.text.trim().is_empty() {
        return Err("Texte vide : rien à traduire".into());
    }
    let src = req.source_lang.unwrap_or_else(|| "auto".into());
    Ok(TranslationResult {
        translated_text: req.text,
        detected_source_lang: src,
        target_lang: req.target_lang,
    })
}
