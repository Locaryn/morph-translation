(function () {
  "use strict";

  const CSS = `
:host { display: block; width: 100%; color: var(--text, #e8edf5); font-family: inherit; box-sizing: border-box; }
* { box-sizing: border-box; }
.panel-container { width: 100%; max-width: 920px; margin: 0 auto; display: flex; flex-direction: column; gap: 16px; }
.header-card {
  display: flex; align-items: center; justify-content: space-between; padding: 16px 20px;
  background: var(--surface, rgba(255, 255, 255, 0.035)); border: 1px solid var(--border, rgba(255, 255, 255, 0.1));
  border-radius: var(--radius, 12px);
}
.title-wrap { display: flex; align-items: center; gap: 12px; }
.icon-box {
  width: 40px; height: 40px; border-radius: 10px; background: rgba(var(--accent-rgb, 110, 168, 254), 0.15);
  color: var(--accent, #6ea8fe); display: grid; place-items: center; font-size: 20px;
}
.title { font-size: 16px; font-weight: 700; color: var(--text, #e8edf5); }
.subtitle { font-size: 12px; color: var(--text-faint, #96a3b8); margin-top: 2px; }
.badge {
  display: inline-flex; align-items: center; padding: 4px 10px; border-radius: 99px; font-size: 11px;
  font-weight: 600; background: rgba(101, 211, 145, 0.12); color: #65d391; border: 1px solid rgba(101, 211, 145, 0.25);
}
.field-card {
  display: flex; flex-direction: column; gap: 10px; background: var(--surface, rgba(255, 255, 255, 0.035));
  border: 1px solid var(--border, rgba(255, 255, 255, 0.1)); border-radius: var(--radius, 12px); padding: 16px;
}
.label { font-size: 11px; font-weight: 700; color: var(--text-dim, #94a3b8); text-transform: uppercase; letter-spacing: 0.06em; }
.textarea, .select {
  width: 100%; border: 1px solid var(--border, rgba(255, 255, 255, 0.14)); border-radius: var(--radius-sm, 8px);
  background: var(--bg, rgba(0, 0, 0, 0.25)); color: inherit; padding: 10px 12px; font: inherit; font-size: 13px; outline: none;
}
.textarea { min-height: 90px; resize: vertical; }
.btn-primary {
  width: 100%; padding: 12px; background: var(--accent, #6ea8fe); color: #0b101b; border: none;
  border-radius: var(--radius-sm, 8px); font-weight: 700; font-size: 14px; cursor: pointer;
}
.btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }
`;

  class LocarynTranslationPanel extends HTMLElement {
    constructor() {
      super();
      this.attachShadow({ mode: "open" });
      this.text = "";
      this.targetLang = "en";
      this.isTranslating = false;
      this.result = "";
    }
    connectedCallback() { this.render(); }

    async translate() {
      if (!this.text.trim() || this.isTranslating) return;
      this.isTranslating = true;
      this.render();
      try {
        const bridge = window.locaryn || window.LocarynPluginAPI;
        if (bridge && bridge.invokeExtensionTool) {
          const res = await bridge.invokeExtensionTool("translate_text", {
            text: this.text,
            target_lang: this.targetLang
          });
          const parsed = typeof res === "string" ? JSON.parse(res) : res;
          this.result = parsed.translated_text || this.text;
        } else {
          this.result = this.text;
        }
      } catch (err) {
        alert("Erreur de traduction: " + err);
      } finally {
        this.isTranslating = false;
        this.render();
      }
    }

    render() {
      this.shadowRoot.innerHTML = `
        <style>${CSS}</style>
        <div class="panel-container">
          <div class="header-card">
            <div class="title-wrap">
              <div class="icon-box">🌐</div>
              <div>
                <div class="title">Studio Traduction Multilingue</div>
                <div class="subtitle">Traduction instantanée via NLLB-200 & Opus-MT</div>
              </div>
            </div>
            <div class="badge">Actif</div>
          </div>

          <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 12px;">
            <div class="field-card">
              <label class="label">Langue cible</label>
              <select class="select" id="tr-lang">
                <option value="en" ${this.targetLang === "en" ? "selected" : ""}>Anglais (English)</option>
                <option value="fr" ${this.targetLang === "fr" ? "selected" : ""}>Français</option>
                <option value="es" ${this.targetLang === "es" ? "selected" : ""}>Espagnol (Español)</option>
                <option value="de" ${this.targetLang === "de" ? "selected" : ""}>Allemand (Deutsch)</option>
                <option value="zh" ${this.targetLang === "zh" ? "selected" : ""}>Chinois (中文)</option>
                <option value="ja" ${this.targetLang === "ja" ? "selected" : ""}>Japonais (日本語)</option>
              </select>
            </div>
          </div>

          <div class="field-card">
            <label class="label">Texte source</label>
            <textarea class="textarea" id="tr-text" placeholder="Entrez le texte à traduire...">${this.text}</textarea>
          </div>

          <button class="btn-primary" id="tr-btn" ${this.isTranslating || !this.text.trim() ? "disabled" : ""}>
            ${this.isTranslating ? "Traduction en cours..." : "Traduire le texte"}
          </button>

          ${this.result ? `
            <div class="field-card" style="margin-top: 10px;">
              <label class="label">Traduction (${this.targetLang.toUpperCase()})</label>
              <div style="font-size: 14px; line-height: 1.5; color: var(--text); padding: 8px 0;">
                ${this.result}
              </div>
            </div>
          ` : ""}
        </div>
      `;

      const textEl = this.shadowRoot.querySelector("#tr-text");
      if (textEl) {
        textEl.addEventListener("input", (e) => {
          this.text = e.target.value;
          const btn = this.shadowRoot.querySelector("#tr-btn");
          if (btn) btn.disabled = !this.text.trim() || this.isTranslating;
        });
      }

      const langEl = this.shadowRoot.querySelector("#tr-lang");
      if (langEl) {
        langEl.addEventListener("change", (e) => { this.targetLang = e.target.value; });
      }

      const btn = this.shadowRoot.querySelector("#tr-btn");
      if (btn) btn.addEventListener("click", () => this.translate());
    }
  }

  if (!customElements.get("locaryn-translation-panel")) {
    customElements.define("locaryn-translation-panel", LocarynTranslationPanel);
  }
})();
