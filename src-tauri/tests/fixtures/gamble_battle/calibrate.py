"""从原始图包裁剪 720p 模板及去除个人信息的识别测试截图。

用法：python calibrate.py <已解压图包目录> [已解码的视频帧目录]
需要 Pillow。脚本相对于自身定位仓库资源目录。
"""

import sys
from pathlib import Path

from PIL import Image

source = Path(sys.argv[1])
destination = Path(__file__).resolve().parent
templates = destination.parents[3] / "resources" / "templates" / "百趣演武"
templates.mkdir(parents=True, exist_ok=True)


def screenshot(time):
    return Image.open(source / f"PixPin_2026-10-29_17-{time}.png").convert("RGB")


full = screenshot("16-22")
for rank, slot in [(1, 0), (2, 3), (3, 1), (4, 2), (5, 4)]:
    x = 105 + 184 * slot
    full.crop((x + 10, 169, x + 35, 200)).save(templates / f"rank-{rank}.png")

for suit, slot in [("bing", 0), ("wan", 1), ("tiao", 2)]:
    x = 105 + 184 * slot
    full.crop((x + 42, 178, x + 65, 201)).save(templates / f"{suit}.png")

screenshot("15-11").crop((122, 275, 218, 298)).save(templates / "empty.png")

for name, time, bounds in [
    ("activity", "15-11", (1138, 106, 1250, 129)),
    ("draw", "15-11", (1198, 533, 1250, 557)),
    ("abandon", "15-27", (854, 643, 949, 670)),
    ("confirm-abandon", "15-45", (440, 346, 839, 370)),
]:
    screenshot(time).crop(bounds).save(templates / f"{name}.png")

for time, name in [
    ("15-11", "empty"), ("15-15", "one"), ("15-18", "two"),
    ("15-21", "three"), ("15-24", "four"),
    ("15-27", "pair"), ("16-22", "straight"),
]:
    original = screenshot(time)
    redacted = Image.new("RGB", original.size)
    # 只保留识别需要的牌头和牌背标记，不保留 UID、昵称等画面内容。
    for bounds in [(100, 158, 1015, 218), (100, 270, 1015, 305)]:
        redacted.paste(original.crop(bounds), bounds[:2])
    redacted.save(destination / f"{name}.png")

if len(sys.argv) > 2:
    for second in [26, 30, 46, 66]:
        name = f"video-{second:03d}.png"
        original = Image.open(Path(sys.argv[2]) / name).convert("RGB")
        redacted = Image.new("RGB", original.size)
        for bounds in [(100, 158, 1015, 218), (100, 270, 1015, 305)]:
            redacted.paste(original.crop(bounds), bounds[:2])
        redacted.save(destination / name)
