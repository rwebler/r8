"""Materialize the complete ROM capacity candidate without changing shipped ROM.

Usage: python3 build.py /tmp/video-sound-candidate.asm
Then assemble that file with the repository's r8asm.
"""
from pathlib import Path
import sys

here = Path(__file__).resolve().parent
source = (here.parent.parent / "sys/basic_rom.asm").read_text()
commands = [("SCREEN", 0xA9), ("CLS", 0xAA), ("COLOR", 0xAB), ("PLOT", 0xAC)]
dispatch = "VIDEO_DISPATCH_START:\n"
for name, _ in commands:
    dispatch += (f"    ld cd, KW_{name}\n    call MATCH\n"
                 f"    bne VIDEO_NEXT_{name}\n    jmp VIDEO_{name}\n"
                 f"VIDEO_NEXT_{name}:\n")
dispatch += "VIDEO_DISPATCH_END:\n"
source = source.replace("LONG_47:\n", "LONG_47:\n" + dispatch, 1)
keywords = "VIDEO_KEYWORDS_START:\n"
for name, code in commands:
    keywords += f'KW_{name}:\n    data 0x{code:02X}, "{name}", 0x00\n'
keywords += "VIDEO_KEYWORDS_END:\n"
source = source.replace("TOKEN_TABLE_END:\n", keywords + "TOKEN_TABLE_END:\n", 1)
help_text = ('VIDEO_HELP_START:\n'
             '    data "SCREEN 0/1; CLS; COLOR ink,paper; PLOT x,y", 0x0A\n'
             '    data "PRINT stays serial; video/sound ports: PEEK/POKE", 0x0A\n'
             'VIDEO_HELP_END:\n')
source = source.replace("HELP_TEXT:\n", "HELP_TEXT:\n" + help_text, 1)
spacing = ("VIDEO_SPACING_START:\n    cmp b, 0xA9\n"
           "    bcc VIDEO_SPACING_END\n    cmp b, 0xAD\n"
           "    bcc DETOKEN_KEYWORD_SPACE\nVIDEO_SPACING_END:\n")
source = source.replace("    cmp b, 0xA7\n", spacing + "    cmp b, 0xA7\n", 1)
source += "\n" + (here / "routines.asm").read_text()
Path(sys.argv[1]).write_text(source)
