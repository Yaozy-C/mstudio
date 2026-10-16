"""Offline regression checks for visual/VFX methods and authored case contracts.

This does not invoke an agent, inspect pixels, call a provider or prove aesthetics.
Run: python3 scripts/test_visual_methods.py
The full repository skill validator is skipped on partial source snapshots.
"""
from copy import deepcopy
import json
from pathlib import Path
import re
import subprocess
import sys
import unittest

ROOT = Path(__file__).resolve().parents[1]
SKILLS = ["creative-ad-director", "image-production", "storyboard-image-production",
          "product-video-production", "storyboard-video-production"]
METHODS = {
    "creative-ad-director": "visual-design.md",
    "image-production": "visual-quality.md",
    "storyboard-image-production": "visual-quality.md",
    "product-video-production": "effects.md",
    "storyboard-video-production": "effects.md",
}
DATA = json.loads((ROOT / "docs/testing/visual-method-cases.json").read_text(encoding="utf-8"))
CASES = DATA["cases"]
REVISION = "visual-vfx-2026-10-16.1"


def read_entry(skill):
    return (ROOT / "skills" / skill / "SKILL.md").read_text(encoding="utf-8")


def read_method(skill):
    return (ROOT / "skills" / skill / "references" / METHODS[skill]).read_text(encoding="utf-8")


def contract_errors(case):
    """Validate authored fixture consistency, not natural-language/model behaviour."""
    errors = []
    missing = set(case["required_inputs"]) - set(case["inputs"])
    conflict = set(case["changes"]) & set(case["protects"])
    unavailable = case["route"] not in case["available_routes"]
    blocked = bool(missing or conflict or unavailable)
    expected = "blocked" if blocked else "ready-for-authorized-test"
    if case["decision"] != expected:
        errors.append("decision does not match declared inputs/capabilities/scope")
    if blocked and (case["prompt"] or not case["reason"].strip()):
        errors.append("blocked example must explain its gap and omit a final prompt")
    if not blocked and not case["prompt"].strip():
        errors.append("ready example requires authored prompt prose")
    if re.search(r"\{\{|\}\}|\bTODO\b|<INSERT", case["prompt"]):
        errors.append("unresolved placeholder in authored prompt")
    if case["authorized_calls"] < 1:
        errors.append("ready/blocked examples require an explicit test budget")
    if case["submitted_calls"] != 0 or case["media_result"] != "not-run":
        errors.append("authored fixture cannot claim a live media result")
    if case["evidence"] != "authored-fixture-not-agent-output":
        errors.append("fixture provenance must be explicit")
    return errors


class MethodDocuments(unittest.TestCase):
    def test_entries_have_matching_frontmatter(self):
        for skill in SKILLS:
            with self.subTest(skill=skill):
                self.assertRegex(read_entry(skill), rf"\A---\nname: {skill}\n")
                self.assertIn("\ndescription: ", read_entry(skill))

    def test_new_methods_are_linked_inside_the_owning_root(self):
        for skill in SKILLS:
            with self.subTest(skill=skill):
                relative = "references/" + METHODS[skill]
                self.assertIn("](" + relative + ")", read_entry(skill))
                path = ROOT / "skills" / skill / relative
                self.assertTrue(path.is_file())
                self.assertTrue(path.resolve().is_relative_to(
                    (ROOT / "skills" / skill).resolve()))

    def test_revision_can_be_checked_in_runtime_text(self):
        for skill in SKILLS:
            with self.subTest(skill=skill):
                self.assertIn(REVISION, read_entry(skill))
                self.assertIn(REVISION, read_method(skill))

    def test_resources_are_english_portable_and_within_size_limits(self):
        for skill in SKILLS:
            for text in (read_entry(skill), read_method(skill)):
                with self.subTest(skill=skill):
                    self.assertLessEqual(len(text.splitlines()), 300)
                    self.assertLessEqual(len(text.encode()), 200_000)
                    self.assertIsNone(re.search(r"[\u3400-\u4dbf\u4e00-\u9fff]", text))
                    self.assertNotRegex(text, r"/Users/|/home/|~/.codex")

    def test_isolated_image_method_copies_match(self):
        self.assertEqual(read_method("image-production"),
                         read_method("storyboard-image-production"))

    def test_isolated_video_method_copies_match(self):
        self.assertEqual(read_method("product-video-production"),
                         read_method("storyboard-video-production"))

    def test_essential_vfx_rules_are_inline_for_quick_generation(self):
        for skill in ("product-video-production", "storyboard-video-production"):
            with self.subTest(skill=skill):
                entry = read_entry(skill)
                for rule in ("protected region", "visible progression", "final state",
                             "actual submitted prompt", "actual capabilities",
                             "before", "paid call", "unverified"):
                    self.assertIn(rule, entry)

    def test_explicit_transformations_are_not_blanket_forbidden(self):
        for skill in ("product-video-production", "storyboard-video-production"):
            with self.subTest(skill=skill):
                self.assertIn("unrequested morphing", read_entry(skill))
                self.assertIn("declared target", read_entry(skill))
                self.assertIn("unchanged silhouette", read_entry(skill))

    def test_board_default_and_explicit_still_exceptions_remain(self):
        self.assertIn("one temporal 2 by 2 action board", read_entry(SKILLS[2]))
        self.assertIn("exact control frame retain their requested format", read_entry(SKILLS[2]))
        self.assertIn("do not silently replace boards", read_entry(SKILLS[4]))
        self.assertIn("Never send the whole board as an exact first/last frame",
                      read_entry(SKILLS[4]))

    def test_board_core_does_not_override_explicit_standalone_tasks(self):
        core = (ROOT / "skills/storyboard-video-production/CORE.md").read_text(encoding="utf-8")
        self.assertIn("For a selected shot-board task", core)
        self.assertIn("standalone effect or exact-control task keeps", core)
        self.assertIn("Do not require a board for it", core)

    def test_methods_have_bounded_review_and_honesty_rules(self):
        for skill in SKILLS[1:]:
            with self.subTest(skill=skill):
                text = read_method(skill).lower()
                self.assertIn("budget", text)
                self.assertIn("unverified", text)
                self.assertIn("actual", text)
        self.assertIn("Sampled frames", read_method("product-video-production"))
        self.assertRegex(read_method("creative-ad-director"), r"not (?:simply )?a blue cast")

    @unittest.skipUnless((ROOT / "skills/video-analysis/SKILL.md").is_file(),
                         "full-repository validator requires all nine source packages")
    def test_original_full_repository_skill_validator(self):
        result = subprocess.run([sys.executable, "scripts/check_skills.py"], cwd=ROOT,
                                capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


class AuthoredCaseContracts(unittest.TestCase):
    def test_nine_distinct_synthetic_cases(self):
        self.assertEqual(len(CASES), 9)
        self.assertEqual(len({case["id"] for case in CASES}), 9)
        self.assertTrue(all(case["skill"] in SKILLS for case in CASES))

    def test_all_authored_examples_are_internally_consistent(self):
        for case in CASES:
            with self.subTest(case=case["id"]):
                self.assertEqual(contract_errors(case), [])

    def test_missing_transition_endpoint_cannot_be_marked_ready(self):
        case = deepcopy(CASES[5])
        case["decision"] = "ready-for-authorized-test"
        self.assertTrue(contract_errors(case))

    def test_whole_melt_preservation_conflict_cannot_be_marked_ready(self):
        case = deepcopy(CASES[4])
        case["decision"] = "ready-for-authorized-test"
        self.assertTrue(contract_errors(case))

    def test_unsupported_board_route_cannot_be_marked_ready(self):
        case = deepcopy(CASES[6])
        case["decision"] = "ready-for-authorized-test"
        self.assertTrue(contract_errors(case))

    def test_reference_mode_cannot_be_labelled_video_editing(self):
        case = deepcopy(CASES[7])
        case["decision"] = "ready-for-authorized-test"
        self.assertTrue(contract_errors(case))

    def test_missing_end_control_blocks_frost_example(self):
        case = deepcopy(CASES[2])
        del case["inputs"]["end"]
        self.assertTrue(contract_errors(case))

    def test_ready_prompt_cannot_keep_unresolved_placeholders(self):
        case = deepcopy(CASES[0])
        case["prompt"] += " {{LIGHT_DIRECTION}}"
        self.assertTrue(contract_errors(case))

    def test_fixture_cannot_claim_it_generated_media(self):
        case = deepcopy(CASES[0])
        case["submitted_calls"] = 1
        case["media_result"] = "aesthetic-pass"
        self.assertTrue(contract_errors(case))

    def test_ready_example_requires_prompt_and_authorization(self):
        case = deepcopy(CASES[0])
        case["prompt"] = ""
        case["authorized_calls"] = 0
        self.assertEqual(len(contract_errors(case)), 2)


if __name__ == "__main__":
    print("OFFLINE ONLY: documents + authored case contracts; no agent/model/media run.",
          flush=True)
    unittest.main(verbosity=2)
