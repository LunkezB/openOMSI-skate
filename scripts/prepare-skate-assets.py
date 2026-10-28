"""Convert a user's extracted Skate 3 disc for the in-process openOMSI bridge."""
import argparse
from pathlib import Path
import sys
import tempfile

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "vendor" / "skate"))


def convert(xex, out):
    xex, out = xex.resolve(), out.resolve()
    if xex.name.lower() != "default.xex" or not xex.is_file():
        raise ValueError("Select default.xex from your extracted Xbox 360 Skate 3 copy.")
    if out.exists():
        raise ValueError("Output already exists; choose a new directory.")
    if not out.parent.is_dir():
        raise ValueError("The output's parent directory must already exist.")
    required = ["data/big/miscload.big", "data/big/miscboot.big", "data/big/db.big", "data/content/createacharacter.big"]
    for relative in required:
        if not (xex.parent / relative).is_file():
            raise ValueError(f"Missing {relative} beside default.xex.")
    if out.is_relative_to(xex.parent) or xex.is_relative_to(out):
        raise ValueError("Choose an output directory outside the original game.")
    from tools.asset_pipeline import asset_exports
    with tempfile.TemporaryDirectory(prefix="omsi-skate-", dir=out.parent) as temporary:
        temporary = Path(temporary)
        stage = temporary / "stage"
        work = temporary / "work"
        stage.mkdir()
        work.mkdir()
        with (stage / "conversion.log").open("w", encoding="utf-8") as log:
            report = lambda message: print(message, flush=True)
            converted = asset_exports.core(xex.parent, stage, work, report, log)
            asset_exports.character(xex.parent, stage, work, report, log, converted)
        for relative in ["private/game.json", "private/skater.glb", "private/stock/physics-skeletons.json", "private/stock/skater-collections.json"]:
            if not (stage / "assets" / relative).is_file():
                raise RuntimeError(f"Conversion did not produce {relative}")
        if out.exists():
            raise ValueError("Output was created during conversion; refusing to overwrite it.")
        stage.rename(out)
    print(f"Ready. Set OMSI_SKATE_ASSETS to {out / 'assets'}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--xex", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    try:
        convert(args.xex, args.out)
    except (ValueError, RuntimeError, OSError, ImportError) as error:
        parser.exit(1, f"Conversion failed: {error}\n")


if __name__ == "__main__":
    main()
