import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
REPORT = ROOT / "target/llvm-cov/coverage.json"
METRICS = ("regions", "functions", "lines")

subprocess.run(
    [
        "cargo",
        "llvm-cov",
        "--workspace",
        "--all-targets",
        "--html",
        "--ignore-filename-regex",
        "main.rs",
    ],
    cwd=ROOT,
    check=True,
)
subprocess.run(
    [
        "cargo",
        "llvm-cov",
        "report",
        "--json",
        "--summary-only",
        "--output-path",
        str(REPORT),
        "--ignore-filename-regex",
        "main.rs",
    ],
    cwd=ROOT,
    check=True,
)

report = json.loads(REPORT.read_text())
coverage = report["data"][0]
files = [
    file
    for file in coverage["files"]
    if "crates" in Path(file["filename"]).parts
    and "src" in Path(file["filename"]).parts
    and file["filename"].endswith(".rs")
]
if not files:
    raise RuntimeError("Coverage report contains no Rust source files")

failures = []
warnings = []
checks = [
    (f"workspace {metric}", coverage["totals"][metric]["percent"])
    for metric in METRICS
]
checks.extend(
    (
        f"{file['filename']} {metric}",
        file["summary"][metric]["percent"],
    )
    for file in files
    for metric in METRICS
)

for label, percent in checks:
    if percent < 50:
        failures.append(f"{label}: {percent:.2f}% (minimum 50%)")
    elif percent < 70:
        warnings.append(f"{label}: {percent:.2f}% (below 70%)")

for warning in warnings:
    print(f"::warning::{warning}")
for failure in failures:
    print(f"::error::{failure}")

if failures:
    raise SystemExit(1)

for metric in METRICS:
    print(f"Workspace {metric}: {coverage['totals'][metric]['percent']:.2f}%")
print(f"Files below 70%: {len(warnings)} warning(s)")
