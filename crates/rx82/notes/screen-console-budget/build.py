"""Copy the integrated native BASIC source for independent measurement."""
from pathlib import Path
import sys

here = Path(__file__).resolve().parent
source = here.parents[1] / "sys/basic_rom.asm"
Path(sys.argv[1]).write_bytes(source.read_bytes())
