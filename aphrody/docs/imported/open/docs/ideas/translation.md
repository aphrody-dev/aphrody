# Translation and language identification: Google's offerings

Facts read from a crawl of Google's documentation, made with `tools/site-mirror --follow --prefix` on the test host. The mirror (screenshots, rendered DOM, every response) is in the mirror's output directory on that host, one folder per crawl:
- `cloud-translate`: docs.cloud.google.com/translate/
- `chrome-ai`: developer.chrome.com/docs/ai/
- `mlkit-translate`: developers.google.com/ml-kit/language/
- `gemma-docs`: ai.google.dev/gemma/docs/

The crawl did not cover translate.google.com, the consumer product.

Under R22/D20 these are backends behind the machine-translation adapter, not picks. Locales are the catalogs present, and the MT model is the one discovered at run time.

## Backends

| Backend | Where it runs | What it offers | Constraints | Source |
|---|---|---|---|---|
| Cloud Translation, Basic | Google Cloud | Standard NMT model, plug and play | Cloud account, per-character billing | `cloud-translate/translate__docs__editions` |
| Cloud Translation, Advanced (v3) | Google Cloud | NMT plus the Translation LLM (TLLM); customization of both; Adaptive Translation from a few example pairs, with multi-sentence context; glossaries; document, batch and real-time translation | same | `translate__docs__editions`, `translate__docs__advanced__translating-text-v3` |
| Vertex AI Model Garden and Studio | Google Cloud | NMT custom models, TLLM supervised fine-tuning, other foundation models | build your own MLOps pipeline | `translate__docs__editions` |
| Chrome built-in Translator API | the user's browser, on device | `Translator.availability()`, `create()`, `translate()`; language packs downloaded on demand | model download behind user activation; top-level windows and same-origin iframes by default, cross-origin iframes through Permission Policy; every pair reports `downloadable` until a site creates it (privacy); built-in AI runs on Windows 10/11, macOS 13+, Linux and ChromeOS Platform 16389+ on Chromebook Plus, not yet on Android, iOS or other ChromeOS | `chrome-ai/docs__ai__translator-api`, `docs__ai__get-started` |
| Chrome built-in Language Detector API | on device | `LanguageDetector.detect()` with a confidence per language | same activation and iframe rules | `chrome-ai/docs__ai__language-detection` |
| ML Kit on-device translation | Android and iOS apps | 50+ languages, offline, the same models as Google Translate's offline mode, models downloaded on demand | non-English pairs pivot through English, which can lower quality | `mlkit-translate/ml-kit__language__translation*` |
| ML Kit language identification | Android and iOS apps | 100+ languages, BCP 47 codes, native scripts | none listed | `mlkit-translate/ml-kit__language__identification*` |
| TranslateGemma | local (llama.cpp, Ollama, Transformers) or any host | open weights in 4B, 12B and 27B; 55 languages; fine-tuned from Gemma 3; text and image input (896x896, 256 tokens per image) | 2K-token input context; Gemma Terms of Use; chat template takes one entry per user message with `source_lang_code` and `target_lang_code` (ISO 639-1 or regional); greedy decoding in the examples; custom prompts unsupported | `gemma-docs/gemma__docs__releases`, model card huggingface.co/google/translategemma-12b-it |

## Measured

In a desktop Chrome 157, through claude-in-chrome:
- `LanguageDetector` is available on device. One sentence each in fr, en and ja was detected at confidence 1.000, 0.999 and 0.999.
- `Translator.availability()` for fr→en and en→ja returns `downloadable`.

## Fit with the roadmap

- **Local first (D8):**
  - Chrome's Translator and LanguageDetector cost nothing and run on device inside any Chromium-based surface.
  - TranslateGemma runs on our own hosts and covers 55 languages.
  - ML Kit is the on-device path for the Compose and Flutter targets on mobile.
- **Language-ID gate for the hygiene `non-english` rule:** candidates are Chrome's LanguageDetector (through a headless Chromium) and ML Kit language ID on mobile. The detector must report a confidence, because short technical lines are ambiguous.
- **Catalog drafting:** TranslateGemma's 2K-token context means catalogs go in batches of messages, not whole files. Its fixed chat template takes no glossary, so terminology is enforced after translation.
- **Cloud fallback:** Cloud Translation Advanced, for glossaries, Adaptive Translation and document translation, when a provider key is discovered at run time.

## Gaps

- Cloud Translation coverage, from the languages page (`cloud-translate/translate__docs__languages`): the NMT table has 208 rows, the Translation LLM table 127 (any to any within that list; the same list supports Adaptive Translation), and custom models 54. The first pass timed out on 16 pages; all 16 were fetched again, so the crawl holds 202+ pages with no failure.
- No benchmark of these backends on Aphrody's own catalogs yet.
