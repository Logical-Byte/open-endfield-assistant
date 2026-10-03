"""从 EER 模板生成固定 720p 模板，不参与运行时的坐标转换。

用法：uv run --with pillow python scripts/importEssenceTemplates.py /path/to/EER
生成的 PNG 存在 resources 子模块，Rust 清单只保存逻辑名称与路径。无需字体或游戏窗口。
"""

import json
import sys
from pathlib import Path

from PIL import Image


def main() -> None:
    root = Path(__file__).resolve().parent.parent
    source = Path(sys.argv[1]) / "src/endfield_essence_recognizer/templates"
    target = root / "resources/templates/基质"
    target.mkdir(parents=True, exist_ok=True)
    catalog = json.loads((root / "resources/data/essence_catalog.json").read_text())
    entries = [(stat["id"], source / "generated" / f"{stat['id']}.png") for stat in catalog["stats"]]
    entries += [
        ("scene", source / "screenshot/武器基质.png"),
        ("locked", source / "screenshot/已锁定.png"),
        ("unlocked", source / "screenshot/未锁定.png"),
        ("abandoned", source / "screenshot/已弃用.png"),
        ("unabandoned", source / "screenshot/未弃用.png"),
    ]
    for name, path in entries:
        image = Image.open(path).convert("RGB")
        image = image.resize(tuple(round(size * 2 / 3) for size in image.size), Image.Resampling.LANCZOS)
        image.save(target / f"{name}.png")
    lines = ["// 由 scripts/importEssenceTemplates.py 生成。模板由 Session 在运行时懒加载。", "pub(super) const STAT_TEMPLATES: &[(&str, &str)] = &["]
    for stat in catalog["stats"]:
        name = stat["id"]
        lines.append(f'    ("{name}", "基质/{name}.png"),')
    lines.append("];")
    (root / "src-tauri/src/vision/essence_templates.rs").write_text("\n".join(lines) + "\n")
    print(f"已生成 {len(entries)} 个 720p 模板")


if __name__ == "__main__":
    main()
