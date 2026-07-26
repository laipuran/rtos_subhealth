"""maps/default.json → MuJoCo scene XML 转换

生成 tag_map.xml，包含彩色方块表示的 AprilTag 位置。
然后在 scene.xml 中添加 <include file="tag_map.xml"/> 即可。

用法:
  cd ros2_ws && python3 scripts/map_to_mjmodel.py
  输出: ~/unitree_mujoco/unitree_robots/go2/tag_map.xml
"""
import json
import os

MAP_FILE = os.path.join(os.path.dirname(__file__), "..", "config", "maps", "default.json")
OUT_DIR = os.path.expanduser("~/unitree_mujoco/unitree_robots/go2")
OUT_FILE = os.path.join(OUT_DIR, "tag_map.xml")

COLORS = [
    "1 0 0 0.8",    # red
    "0 1 0 0.8",    # green
    "0 0 1 0.8",    # blue
    "1 1 0 0.8",    # yellow
    "1 0 1 0.8",    # magenta
    "0 1 1 0.8",    # cyan
    "1 0.5 0 0.8",  # orange
    "0.5 0 1 0.8",  # purple
]


def main():
    if not os.path.exists(MAP_FILE):
        print(f"[ERROR] Map file not found: {MAP_FILE}")
        return

    with open(MAP_FILE) as f:
        data = json.load(f)

    tags = data.get("tags", {})
    if not tags:
        print("[ERROR] No tags in map file")
        return

    lines = ['<!-- Auto-generated tag map -->']
    for i, (tag_id, tag_info) in enumerate(sorted(tags.items())):
        x = tag_info.get("x", 0)
        y = tag_info.get("y", 0)
        name = tag_info.get("name", f"tag_{tag_id}")
        rgba = COLORS[i % len(COLORS)]
        lines.append(f'  <body name="tag_{tag_id}" pos="{x} {y} 0.01">')
        lines.append(f'    <geom type="box" size="0.1 0.1 0.002" rgba="{rgba}"/>')
        lines.append(f'    <geom type="box" size="0.15 0.001 0.04" rgba="{rgba}" pos="0 0 0.02"/>')
        lines.append(f'    <site name="tag_site_{tag_id}" pos="0 0 0" size="0.01" group="3"/>')
        lines.append('  </body>')

    os.makedirs(OUT_DIR, exist_ok=True)
    with open(OUT_FILE, "w") as f:
        f.write("\n".join(lines) + "\n")

    print(f"[OK] Generated: {OUT_FILE}")
    print(f"     {len(tags)} tags: {', '.join(sorted(tags.keys()))}")
    print()
    print("Next step: Add this line to your scene.xml:")
    print('  <include file="tag_map.xml"/>')


if __name__ == "__main__":
    main()
