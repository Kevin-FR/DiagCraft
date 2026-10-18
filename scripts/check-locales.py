#!/usr/bin/env python3
"""Check all seven catalogues, substitutions, and explicit GUI/engine label keys."""
from pathlib import Path
import json
import re

ROOT = Path(__file__).resolve().parents[1]
LANGUAGES = {"en", "fr", "de", "es", "it", "pt", "zh-Hans"}
LITERAL = r'"(?:[^"\\]|\\.)*"'


def check():
    catalogue = json.loads((ROOT / "assets/locales/messages.json").read_text(encoding="utf-8"))
    for key, row in catalogue.items():
        assert set(row) == LANGUAGES, key
        assert row["fr"] == key, key
        substitutions = sorted(re.findall(r"\{([^{}]+)\}", key))
        for language, translation in row.items():
            assert translation.strip(), (key, language)
            assert sorted(re.findall(r"\{([^{}]+)\}", translation)) == substitutions, (key, language)
    required = set()
    for path in (ROOT / "apps/diagcraft/src").glob("*.rs"):
        if path.name in {"main.rs", "i18n.rs", "ui_tests.rs"}:
            continue
        source = path.read_text(encoding="utf-8").split("#[cfg(test)]")[0]
        if path.name == "object_search_ui.rs":
            for match in re.finditer(r'fn label\(self\).*?\{(.*?)\n    \}', source, re.S):
                required.update(json.loads(literal) for literal in re.findall(LITERAL, match[1]))
        for match in re.finditer(r'(?:\bt|i18n::render)\(\s*(' + LITERAL + ')', source):
            required.add(json.loads(match[1]))
        if path.name == "commands_ui.rs":
            for match in re.finditer(r'command!\(\s*\w+,\s*(' + LITERAL + r'),\s*(' + LITERAL + ')', source):
                required.update(json.loads(match[i]) for i in (1, 2))
        for match in re.finditer(r'self\.status\s*=\s*(' + LITERAL + ')', source):
            required.add(json.loads(match[1]))
    for path in (ROOT / "crates/core/src").glob("*.rs"):
        source = path.read_text(encoding="utf-8")
        for match in re.finditer(r'pub fn label\(self\).*?\{(.*?)\n    \}', source, re.S):
            required.update(json.loads(literal) for literal in re.findall(LITERAL, match[1]))
        if path.name in {"symbols.rs", "pages.rs"}:
            required.update(json.loads(match[1]) for match in re.finditer(r'(?:title|label|category):\s*(' + LITERAL + ')', source))
    missing = sorted(required - catalogue.keys())
    assert not missing, "Missing translations:\n" + "\n".join(missing)
    print(f"Localization check passed: {len(catalogue)} entries × 7 languages; {len(required)} source keys; substitutions preserved.")


if __name__ == "__main__":
    check()
