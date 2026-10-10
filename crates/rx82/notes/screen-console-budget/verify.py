"""Measure and exercise the integrated native console. Outputs go to /tmp."""
from pathlib import Path
import json
import subprocess
import sys
import tempfile

here = Path(__file__).resolve().parent
root = here.parents[3]
out = Path(tempfile.mkdtemp(prefix="rx82-screen-budget-"))
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
subprocess.run([sys.executable, str(here / "build.py"), str(out / "integrated.asm")], check=True)
with (out / "integrated.map").open("w") as mapping:
    subprocess.run([str(out / "measure"), str(out / "integrated.asm"), str(out / "integrated.bin")],
                   stdout=mapping, check=True)
subprocess.run([str(out / "check"), str(out / "integrated.bin")], check=True)
size = (out / "integrated.bin").stat().st_size
assert (out / "integrated.bin").read_bytes() == (here.parent.parent / "sys/basic_rom.bin").read_bytes()
assert size <= 0x2E00, f"ROM exceeds boundary by {size-0x2E00} bytes"
print(f"Integrated ROM {size}; exclusive end {0xC100+size:04X}; free {0x2E00-size} bytes")
print(f"Artifacts: {out}")
