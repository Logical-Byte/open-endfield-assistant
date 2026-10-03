"""从 EER 的资源仓库导入基质判断所需的最小数据集。

用法：python3 scripts/importEssenceCatalog.py /path/to/eer-resource
只使用 Python 标准库，生成结果随 OEA 提交，运行时无需 EER。
"""

import json
import sys
from pathlib import Path


def main() -> None:
    source = Path(sys.argv[1]) / "data" / "v2"
    stats = json.loads((source / "EssenceStat.json").read_text())
    weapons = json.loads((source / "Weapon.json").read_text())
    catalog = {
        "stats": [
            {"id": key, "name": value["name"], "kind": value["type"].lower()}
            for key, value in sorted(stats.items())
        ],
        "weapons": [
            {
                "id": key,
                "name": value["name"],
                "rarity": value["rarity"],
                "stats": [value[f"stat{i}_id"] for i in range(1, 4)],
            }
            for key, value in sorted(weapons.items())
        ],
    }
    target = Path(__file__).resolve().parent.parent / "resources/data/essence_catalog.json"
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(json.dumps(catalog, ensure_ascii=False, indent=2) + "\n")
    print(f"已导入 {len(catalog['stats'])} 个属性、{len(catalog['weapons'])} 把武器到 {target}")


if __name__ == "__main__":
    main()
