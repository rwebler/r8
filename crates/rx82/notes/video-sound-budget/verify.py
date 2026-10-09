"""Build, measure, and execute the isolated candidate. Outputs go to a temp folder."""
from pathlib import Path
import json
import subprocess
import sys
import tempfile

here = Path(__file__).resolve().parent
root = here.parents[3]
out = Path(tempfile.mkdtemp(prefix="rx82-video-budget-"))
build = subprocess.run(
    ["cargo", "build", "--offline", "--lib", "-p", "rx82", "-p", "r8asm",
     "--message-format=json"], cwd=root, text=True, stdout=subprocess.PIPE, check=True)
libs = {}
for line in build.stdout.splitlines():
    item = json.loads(line)
    if item.get("reason") == "compiler-artifact":
        for filename in item.get("filenames", []):
            if filename.endswith(".rlib"):
                libs[item["target"]["name"]] = Path(filename)
for tool, library in [("measure", "r8asm"), ("check", "rx82")]:
    subprocess.run(["rustc", "--edition=2024", str(here / f"{tool}.rs"),
                    "--extern", f"{library}={libs[library]}", "-L",
                    f"dependency={libs[library].parent / 'deps'}", "-o", str(out / tool)], check=True)
subprocess.run([sys.executable, str(here / "build.py"), str(out / "candidate.asm")], check=True)
for label, source in [("baseline", here.parent.parent / "sys/basic_rom.asm"),
                      ("candidate", out / "candidate.asm")]:
    with (out / f"{label}.map").open("w") as mapping:
        subprocess.run([str(out / "measure"), str(source), str(out / f"{label}.bin")],
                       stdout=mapping, check=True)
subprocess.run([str(out / "check"), str(out / "candidate.bin")], check=True)
base = (out / "baseline.bin").stat().st_size
size = (out / "candidate.bin").stat().st_size
assert (out / "baseline.bin").read_bytes() == (here.parent.parent / "sys/basic_rom.bin").read_bytes()
assert size <= 0x2E00
print(f"Baseline {base}; candidate {size}; added {size-base}; free {0x2E00-size} bytes")
print(f"Artifacts: {out}")
