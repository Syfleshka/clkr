#!/usr/bin/env python3
"""Regenerate assets/phosphor-subset.ttf from src/ui/icons.rs.

Usage: python3 tools/subset_icons.py path/to/Phosphor.ttf
(Phosphor.ttf ships with the `egui-phosphor` crate or https://github.com/phosphor-icons/web.)
Requires: pip install fonttools
"""
import pathlib
import re
import sys

from fontTools import subset

root = pathlib.Path(__file__).resolve().parent.parent
codepoints = sorted({int(cp, 16) for cp in re.findall(r'\\u\{([0-9A-Fa-f]+)\}', (root / "src/ui/icons.rs").read_text())})

options = subset.Options()
options.layout_features = []
options.name_IDs = ["*"]
options.notdef_outline = True
font = subset.load_font(sys.argv[1], options)
subsetter = subset.Subsetter(options)
subsetter.populate(unicodes=codepoints)
subsetter.subset(font)
out = root / "assets/phosphor-subset.ttf"
subset.save_font(font, str(out), options)
print(f"{len(codepoints)} glyphs -> {out} ({out.stat().st_size} bytes)")
