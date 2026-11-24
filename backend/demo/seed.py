import csv
import sys
import time
import zipfile
from pathlib import Path

VECTOR_DIM = 384


def main(src: Path, out: Path) -> None:
    date = time.strftime("%y%m%d")
    vector = " ".join(["0"] * VECTOR_DIM)
    lines = []

    with src.open(newline="", encoding="utf-8") as f:
        for row in csv.DictReader(f, delimiter="\t"):
            content = row["content"]
            fields = [
                row["url"],
                row["title"],
                row["description"],
                content,
                "",
                "",
                row["keywords"],
                "true",
                "",
                row["lang"],
                row["loc"],
                "[0.8, 0.8, 0.8, 0.8]",
                "0.3",
                date,
                "5",
                "true",
                "true" if len(content.split()) >= 500 else "false",
                vector,
            ]
            lines.append("<-->".join(fields))

    out.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
        z.writestr("docs.txt", "\n".join(lines) + "\n")

    print(f"Seeded {len(lines)} sample results into {out}")


if __name__ == "__main__":
    main(Path(sys.argv[1]), Path(sys.argv[2]))
