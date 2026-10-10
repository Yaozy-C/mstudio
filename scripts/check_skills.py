"""Check repository-owned skill metadata, local links and portable resources."""
from pathlib import Path
import re
import sys
from urllib.parse import unquote


def slug(heading):
    heading = re.sub(r"[^\w\s-]", "", heading.lower())
    return re.sub(r"\s", "-", heading)


EXPECTED = {"video-analysis", "creative-concepts", "ad-script", "creative-ad-director", "image-production", "product-video-production", "storyboard-image-production", "storyboard-video-production", "video-editing"}


def validate(root):
    errors = []
    expected = EXPECTED
    folders = {p.name for p in root.iterdir() if p.is_dir()}
    if folders != expected:
        errors.append(f"skill folders differ: {folders ^ expected}")
    for name in expected:
        path = root / name / "SKILL.md"
        if not path.is_file():
            errors.append(f"missing {path}")
            continue
        text = path.read_text()
        match = re.match(r"\A---\n(.*?)\n---\n", text, re.S)
        if not match or not re.search(rf"^name: {re.escape(name)}$", match[1], re.M):
            errors.append(f"invalid name/frontmatter: {path}")
        if not match or not re.search(r"^description: \S.+$", match[1], re.M):
            errors.append(f"missing description: {path}")
    for path in root.rglob("*"):
        if path.is_symlink():
            errors.append(f"symlink not portable: {path}")
            continue
        if not path.is_file():
            continue
        if path.suffix != ".md":
            errors.append(f"unexpected resource: {path}")
            continue
        text = path.read_text()
        if re.search(r"[\u3400-\u4dbf\u4e00-\u9fff]", text):
            errors.append(f"skill resources must be authored in English: {path}")
        for token in ("/Users/", "/home/", "CODEX_HOME", "~/.codex", "本用户"):
            if token in text:
                errors.append(f"personal/external dependency {token}: {path}")
        for link in re.findall(r"\]\(([^)]+)\)", text):
            if link.startswith(("https://", "http://")):
                continue
            target, _, anchor = unquote(link).partition("#")
            resolved = (path.parent / target).resolve() if target else path.resolve()
            if not resolved.is_relative_to((root / path.relative_to(root).parts[0]).resolve()) or not resolved.is_file():
                errors.append(f"missing/escaping reference: {path}: {link}")
                continue
            if anchor:
                headings = re.findall(r"^#{1,6} (.+)$", resolved.read_text(), re.M)
                if anchor not in {slug(h) for h in headings}:
                    errors.append(f"missing anchor: {path}: {link}")
    return errors


if __name__ == "__main__":
    root = Path(__file__).resolve().parents[1] / "skills"
    errors = validate(root)
    if errors:
        print("\n".join(errors))
        sys.exit(1)
    print(f"Skills: {len(EXPECTED)} packages, {len(list(root.rglob('*.md')))} portable Markdown resources; links valid")
