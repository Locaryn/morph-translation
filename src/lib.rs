//! Traduire, par le modèle qui tourne déjà sur la machine.
//!
//! Aucun service de traduction n'est appelé : le texte ne sort pas. C'est le
//! moteur d'inférence local qui traduit, avec une consigne étroite.
//!
//! Cette consigne est tout le travail. Un modèle à qui l'on dit « traduis ceci »
//! répond volontiers « Voici la traduction : … », ajoute une note sur un choix
//! de mot, ou demande une précision. Rien de tout cela n'est une traduction, et
//! le programme qui reçoit la réponse ne sait pas la distinguer du texte. On lui
//! dit donc de ne rendre que la traduction — et on nettoie ce qui passe quand
//! même.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ── Réglages ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Adresse du serveur compatible OpenAI.
    #[serde(default = "endpoint_par_defaut")]
    pub endpoint: String,
    /// Modèle à employer. Vide : celui que le serveur a déjà chargé.
    #[serde(default)]
    pub model: String,
    /// Température. Basse par défaut : une traduction n'a pas à être créative,
    /// et la variété se paie ici en contresens.
    #[serde(default = "temperature_par_defaut")]
    pub temperature: f32,
}

fn endpoint_par_defaut() -> String {
    "http://127.0.0.1:8080".into()
}
fn temperature_par_defaut() -> f32 {
    0.2
}

impl Default for Config {
    fn default() -> Self {
        Self {
            endpoint: endpoint_par_defaut(),
            model: String::new(),
            temperature: temperature_par_defaut(),
        }
    }
}

pub fn config() -> Config {
    let Some(p) = std::env::var("LOCARYN_EXTENSION_CONFIG_FILE")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
    else {
        return Config::default();
    };
    std::fs::read_to_string(p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

// ── Demande et réponse ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationRequest {
    pub text: String,
    /// Langue de départ. Absente : devinée du texte.
    #[serde(default)]
    pub source_lang: Option<String>,
    /// Langue d'arrivée, en code ISO ou en toutes lettres.
    pub target_lang: String,
    /// Modèle, s'il doit différer du réglage.
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationResult {
    pub translated_text: String,
    pub detected_source_lang: String,
    /// Vrai quand la langue de départ a été devinée plutôt que déclarée.
    /// L'heuristique se fonde sur les écritures et les accents : une phrase
    /// française sans accent passe pour anglaise. Le dire permet à l'appelant
    /// de fournir la langue si le résultat surprend.
    pub source_was_guessed: bool,
    pub target_lang: String,
    pub model: String,
}

// ── Traduction ──────────────────────────────────────────────────────────────

/// Traduire un texte.
pub async fn translate_text(req: TranslationRequest) -> Result<TranslationResult, String> {
    if req.text.trim().is_empty() {
        return Err("Texte vide : rien à traduire.".into());
    }
    let cible = req.target_lang.trim();
    if cible.is_empty() {
        return Err("La langue d'arrivée doit être précisée.".into());
    }

    let cfg = config();
    let declaree = req
        .source_lang
        .as_deref()
        .map(str::trim)
        .filter(|l| !l.is_empty() && *l != "auto")
        .map(str::to_string);
    let devinee = declaree.is_none();
    let source = declaree
        .clone()
        .unwrap_or_else(|| detect_language(&req.text).to_string());

    // Traduire vers la langue déjà écrite ne veut rien dire. Mais ce raccourci
    // ne vaut que sur une langue **déclarée** : l'heuristique se fonde sur les
    // accents, et une phrase française sans accent passe pour anglaise. Sur une
    // langue devinée, court-circuiter rendrait le texte inchangé — c'est
    // précisément ce que faisait la version factice de ce morph.
    if !devinee && meme_langue(&source, cible) {
        return Ok(TranslationResult {
            translated_text: req.text.clone(),
            detected_source_lang: source,
            source_was_guessed: false,
            target_lang: cible.to_string(),
            model: "aucun".into(),
        });
    }

    // La langue de départ n'entre dans la consigne que si elle a été
    // **déclarée**. La deviner puis l'imposer au modèle est le pire des deux
    // mondes : l'heuristique se fonde sur les accents, donc une phrase
    // française sans accent passe pour anglaise — et le modèle, à qui l'on
    // demande alors de traduire de l'anglais vers l'anglais, rend le texte
    // intact. Le modèle reconnaît la langue bien mieux que ces règles ; on le
    // laisse faire et on ne lui dit que la destination.
    let depuis = match &declaree {
        Some(l) => format!("depuis {l} "),
        None => String::new(),
    };
    let consigne = format!(
        "Tu traduis {depuis}vers {cible}. Tu rends la traduction, et rien d'autre : \
         pas de préambule, pas de note du traducteur, pas de variante entre parenthèses, \
         pas de guillemets ajoutés. Tu gardes la mise en forme du texte d'origine — les \
         retours à la ligne, les listes, la ponctuation. Tu ne traduis pas les noms \
         propres, les noms de fichiers, les chemins, ni le code."
    );

    let modele = req
        .model
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .unwrap_or(cfg.model.trim());

    let corps = serde_json::json!({
        "model": if modele.is_empty() { "local" } else { modele },
        "temperature": cfg.temperature,
        "messages": [
            { "role": "system", "content": consigne },
            { "role": "user", "content": req.text }
        ]
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!(
            "{}/v1/chat/completions",
            cfg.endpoint.trim_end_matches('/')
        ))
        .timeout(std::time::Duration::from_secs(300))
        .json(&corps)
        .send()
        .await
        .map_err(|_| {
            "Le moteur d'inférence ne répond pas. Démarrez-le, puis relancez la traduction."
                .to_string()
        })?;

    if !resp.status().is_success() {
        let statut = resp.status();
        let detail = resp.text().await.unwrap_or_default();
        return Err(format!(
            "Le moteur a refusé la demande ({statut}){}",
            if detail.trim().is_empty() {
                String::new()
            } else {
                format!(" : {}", tronquer(&detail, 200))
            }
        ));
    }

    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("réponse illisible du moteur : {e}"))?;
    let brut = v
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or("Le moteur n'a rien traduit.")?;

    Ok(TranslationResult {
        translated_text: nettoyer(brut),
        detected_source_lang: source,
        source_was_guessed: devinee,
        target_lang: cible.to_string(),
        model: if modele.is_empty() {
            "celui du serveur".into()
        } else {
            modele.to_string()
        },
    })
}

/// Retirer ce qu'un modèle ajoute malgré la consigne.
///
/// Trois habitudes tenaces : annoncer la traduction avant de la donner,
/// l'entourer d'un bloc de code, et la mettre entre guillemets. Aucune n'est
/// une traduction, et le programme qui reçoit la réponse ne le devine pas.
fn nettoyer(s: &str) -> String {
    let mut t = s.trim();

    // Un bloc de code entier : on garde l'intérieur.
    if t.starts_with("```") {
        if let Some(fin) = t.rfind("```") {
            if fin > 3 {
                let dedans = &t[3..fin];
                // La première ligne peut porter le nom du langage.
                t = match dedans.split_once('\n') {
                    Some((tete, reste)) if !tete.contains(' ') && tete.len() < 20 => reste,
                    _ => dedans,
                };
                t = t.trim();
            }
        }
    }

    // Un préambule, seulement s'il tient sur la première ligne et qu'il reste
    // du texte derrière : sinon on couperait une traduction qui commence par
    // deux-points.
    const PREAMBULES: &[&str] = &[
        "voici la traduction",
        "voici le texte traduit",
        "traduction :",
        "here is the translation",
        "here's the translation",
        "translation:",
    ];
    if let Some((tete, reste)) = t.split_once('\n') {
        let bas = tete.trim().to_ascii_lowercase();
        if !reste.trim().is_empty() && PREAMBULES.iter().any(|p| bas.starts_with(p)) {
            t = reste.trim();
        }
    }

    // Des guillemets qui enveloppent tout, et qu'on n'a pas demandés.
    let c: Vec<char> = t.chars().collect();
    if c.len() >= 2 {
        let (a, z) = (c[0], c[c.len() - 1]);
        let paire = matches!((a, z), ('"', '"') | ('«', '»') | ('“', '”') | ('\'', '\''));
        // Seulement si aucune autre occurrence à l'intérieur : sinon ce sont
        // des guillemets du texte lui-même.
        if paire && !c[1..c.len() - 1].contains(&a) {
            t = t[a.len_utf8()..t.len() - z.len_utf8()].trim();
        }
    }

    t.to_string()
}

/// Deux désignations de la même langue ? On compare sur les deux premières
/// lettres : « fr », « fra », « français », « French » se rejoignent mal, mais
/// les codes ISO, eux, suffisent au cas courant.
fn meme_langue(a: &str, b: &str) -> bool {
    let n = |s: &str| {
        s.trim()
            .to_ascii_lowercase()
            .chars()
            .take(2)
            .collect::<String>()
    };
    !a.trim().is_empty() && n(a) == n(b)
}

fn tronquer(s: &str, n: usize) -> String {
    let t = s.trim();
    if t.chars().count() <= n {
        return t.to_string();
    }
    t.chars().take(n).collect::<String>() + "…"
}

/// La langue du texte, à la même heuristique que le reste de l'application.
pub fn detect_language(text: &str) -> &'static str {
    let has = |ranges: &[(char, char)]| {
        text.chars()
            .any(|c| ranges.iter().any(|(a, b)| (*a..=*b).contains(&c)))
    };
    if has(&[('\u{4e00}', '\u{9fff}'), ('\u{3400}', '\u{4dbf}')]) {
        return "zh";
    }
    if has(&[('\u{3040}', '\u{309f}'), ('\u{30a0}', '\u{30ff}')]) {
        return "ja";
    }
    if has(&[('\u{ac00}', '\u{d7af}')]) {
        return "ko";
    }
    if has(&[('\u{0600}', '\u{06ff}')]) {
        return "ar";
    }
    if has(&[('\u{0400}', '\u{04ff}')]) {
        return "ru";
    }
    let any = |set: &str| text.chars().any(|c| set.contains(c));
    if any("ãõ") {
        return "pt";
    }
    if any("äöüÄÖÜ") {
        return "de";
    }
    if any("àèéêëçôû") {
        return "fr";
    }
    if any("ñ¿¡") {
        return "es";
    }
    if any("ìòù") {
        return "it";
    }
    "en"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_texte_vide_est_refuse_sans_appeler_le_moteur() {
        let err = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(translate_text(TranslationRequest {
                text: "   ".into(),
                source_lang: None,
                target_lang: "en".into(),
                model: None,
            }))
            .unwrap_err();
        assert!(err.contains("vide"), "{err}");
    }

    #[test]
    fn une_langue_d_arrivee_absente_est_refusee() {
        let err = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(translate_text(TranslationRequest {
                text: "bonjour".into(),
                source_lang: None,
                target_lang: "  ".into(),
                model: None,
            }))
            .unwrap_err();
        assert!(err.contains("arrivée"), "{err}");
    }

    /// Traduire vers la langue déjà écrite ne veut rien dire ; le modèle
    /// répondrait n'importe quoi plutôt que de le signaler.
    #[test]
    fn traduire_vers_sa_propre_langue_rend_le_texte_tel_quel() {
        let r = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(translate_text(TranslationRequest {
                text: "où est la clé".into(),
                source_lang: Some("fra".into()),
                target_lang: "fr".into(),
                model: None,
            }))
            .expect("pas une erreur");
        assert_eq!(r.translated_text, "où est la clé");
        assert_eq!(r.detected_source_lang, "fra");
        assert!(!r.source_was_guessed);
    }

    #[test]
    fn le_preambule_est_retire_quand_il_tient_sur_sa_ligne() {
        assert_eq!(
            nettoyer("Voici la traduction :\nHello world"),
            "Hello world"
        );
        assert_eq!(nettoyer("Translation:\nHello"), "Hello");
        // Une traduction qui commence par deux-points ne doit pas être coupée.
        assert_eq!(
            nettoyer("Attention : ne pas ouvrir"),
            "Attention : ne pas ouvrir"
        );
    }

    #[test]
    fn le_bloc_de_code_est_deballe() {
        assert_eq!(nettoyer("```\nHello\n```"), "Hello");
        assert_eq!(nettoyer("```text\nHello\n```"), "Hello");
    }

    #[test]
    fn les_guillemets_ajoutes_partent_mais_pas_ceux_du_texte() {
        assert_eq!(nettoyer("\"Hello\""), "Hello");
        assert_eq!(nettoyer("« Bonjour »"), "Bonjour");
        // Des guillemets internes : ce sont ceux du texte, on n'y touche pas.
        assert_eq!(nettoyer("\"a\" et \"b\""), "\"a\" et \"b\"");
    }

    #[test]
    fn les_codes_de_langue_se_rejoignent_sur_deux_lettres() {
        assert!(meme_langue("fr", "fra"));
        assert!(meme_langue("EN", "eng"));
        assert!(!meme_langue("fr", "en"));
        assert!(
            !meme_langue("", "fr"),
            "une source vide ne vaut pas égalité"
        );
    }

    /// Une vraie traduction, par le moteur de cette machine.
    /// `cargo test -- --ignored --nocapture`
    #[test]
    #[ignore = "exige un moteur d'inférence en écoute"]
    fn traduit_reellement() {
        let r = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(translate_text(TranslationRequest {
                text: "Le tunnel est sortant : l'ordinateur appelle un relais.".into(),
                source_lang: None,
                target_lang: "en".into(),
                model: None,
            }))
            .expect("la traduction doit aboutir");
        println!(
            "{} -> {} : {}",
            r.detected_source_lang, r.target_lang, r.translated_text
        );
        assert!(!r.translated_text.trim().is_empty());
        assert_ne!(
            r.translated_text.trim(),
            "Le tunnel est sortant : l'ordinateur appelle un relais.",
            "le texte est revenu inchange : le raccourci « meme langue » a mordu"
        );
        assert!(
            !r.translated_text
                .to_lowercase()
                .contains("voici la traduction"),
            "le préambule aurait dû être retiré : {}",
            r.translated_text
        );
    }
}
