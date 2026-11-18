# Lessons

The first delivery was rejected and purged. Causes, and the rules that follow.

| What happened | Rule |
|---|---|
| Hand-made interfaces (CSS, Win32, colour engine, generic Figma mockups) | Google's official components and tokens only; never re-implement |
| Hardcoded values (table rows, colours, model lists) | Data comes from a source (lock, API, generated tokens) |
| Agent fan-out, 90 repositories cloned, many deployments, no plan | A roadmap and a spec agreed before executing |
| Delivery in small pieces "not verified, not compiled" | Deliver complete: built, running, tested, proven |
| Surface checks (READMEs) | Read manifests, code, Context7 and the dependency graph before choosing |
| Playwright chosen without justification while a list of tools was given | Start from the given list and the existing inventory |
| Material Web presented as the web implementation | Check a project's real status (maintenance, archived) before adopting it |
| Machine excuses ("this PC has no ...") | Fix the configuration reproducibly |
| French written straight into docs and reports | English source, multilingual through i18n and a translation stack |
