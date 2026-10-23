#!/usr/bin/env python3
"""Sync repository skill Markdown into an Mstudio database.

The repository files are installation defaults. Mstudio stores the active
copies in `skill_resources`, so editing the repository alone does not change an
existing installation. This utility previews or applies that sync.

Default is a dry run. Use --write after quitting Mstudio. A timestamped backup
is created before any write, and only the selected skill packages are touched.
The write replaces the active text for those packages, so review the dry run
first if the installation has custom edits.
"""
from __future__ import annotations

import argparse
import datetime as dt
import sqlite3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "skills"
DEFAULT_DB = (
    Path.home()
    / "Library"
    / "Application Support"
    / "local.mstudio.canvas"
    / "mstudio.sqlite3"
)


def resources() -> list[tuple[str, str, str]]:
    items: list[tuple[str, str, str]] = []
    for package in sorted(path for path in ROOT.iterdir() if path.is_dir()):
        for path in sorted(package.rglob("*.md")):
            items.append(
                (package.name, path.relative_to(package).as_posix(), path.read_text())
            )
    return items


def backup(db: Path) -> Path:
    target = db.with_name(
        f"{db.name}.before-skill-sync-{dt.datetime.now():%Y%m%d-%H%M%S}"
    )
    source = sqlite3.connect(db)
    destination = sqlite3.connect(target)
    try:
        source.backup(destination)
    finally:
        destination.close()
        source.close()
    return target


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", type=Path, default=DEFAULT_DB)
    parser.add_argument("--skill", action="append", default=[])
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    db = args.db.expanduser()
    if not db.is_file():
        parser.error(f"database not found: {db}")

    selected = set(args.skill)
    items = [row for row in resources() if not selected or row[0] in selected]
    if not items:
        parser.error("no matching skill resources")

    if args.write:
        print("Warning: this replaces the active text for the selected packages.")
        print("Quit Mstudio before writing; restart it afterwards to load the new text.")
        print(f"Backup: {backup(db)}")
        connection = sqlite3.connect(db)
    else:
        connection = sqlite3.connect(f"file:{db}?mode=ro", uri=True)

    added = updated = unchanged = 0
    try:
        with connection:
            for skill, path, text in items:
                row = connection.execute(
                    "SELECT text FROM skill_resources WHERE skill_id=? AND path=?",
                    (skill, path),
                ).fetchone()
                if row is None:
                    added += 1
                    print(f"ADD      {skill}/{path}")
                    if args.write:
                        connection.execute(
                            "INSERT INTO skill_resources(skill_id,path,text) VALUES(?,?,?)",
                            (skill, path, text),
                        )
                elif row[0] != text:
                    updated += 1
                    print(f"UPDATE   {skill}/{path}")
                    if args.write:
                        connection.execute(
                            "UPDATE skill_resources SET text=?, revision=revision+1, "
                            "updated=unixepoch() WHERE skill_id=? AND path=?",
                            (text, skill, path),
                        )
                else:
                    unchanged += 1
    finally:
        connection.close()

    verb = "Applied" if args.write else "Would apply"
    print(
        f"{verb}: {added} added, {updated} updated, {unchanged} unchanged. "
        + ("" if args.write else "Re-run with --write after quitting Mstudio.")
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
