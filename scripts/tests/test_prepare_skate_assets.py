import importlib.util
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("prepare_skate", ROOT / "scripts/prepare-skate-assets.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ConverterBoundaries(unittest.TestCase):
    def test_existing_output_survives(self):
        with tempfile.TemporaryDirectory(prefix="omsi-skate-test-") as temporary:
            root = Path(temporary)
            xex = root / "game/default.xex"
            xex.parent.mkdir()
            xex.touch()
            output = root / "output"
            output.mkdir()
            sentinel = output / "keep.txt"
            sentinel.write_text("keep")
            with self.assertRaisesRegex(ValueError, "already exists"):
                module.convert(xex, output)
            self.assertEqual(sentinel.read_text(), "keep")

    def test_incomplete_disc_does_not_publish_output(self):
        with tempfile.TemporaryDirectory(prefix="omsi-skate-test-") as temporary:
            root = Path(temporary)
            xex = root / "default.xex"
            xex.touch()
            output = root / "result"
            with self.assertRaisesRegex(ValueError, "Missing"):
                module.convert(xex, output)
            self.assertFalse(output.exists())

    def test_writing_into_game_is_refused(self):
        with tempfile.TemporaryDirectory(prefix="omsi-skate-test-") as temporary:
            root = Path(temporary)
            xex = root / "default.xex"
            xex.touch()
            for relative in ["data/big/miscload.big", "data/big/miscboot.big", "data/big/db.big", "data/content/createacharacter.big"]:
                p = root / relative
                p.parent.mkdir(parents=True, exist_ok=True)
                p.touch()
            with self.assertRaisesRegex(ValueError, "outside the original game"):
                module.convert(xex, root / "converted")
            self.assertFalse((root / "converted").exists())


if __name__ == "__main__":
    unittest.main(verbosity=2)
