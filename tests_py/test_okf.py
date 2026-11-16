import unittest
from pathlib import Path
from okf_markdown import convert_transcript_to_okf, validate_okf_document


class TestOkfPython(unittest.TestCase):
    def test_vtt_conversion_and_validation(self):
        vtt = (Path(__file__).parent.parent / "examples/sample.vtt").read_text(encoding="utf-8")
        res = convert_transcript_to_okf(vtt, title="Python Whisper Sync", created_at="2026-11-16T12:00:00Z")
        self.assertEqual(res["speakers"], ["Maya Lin", "Alex Rivera"])
        self.assertEqual(res["turn_count"], 2)
        doc = validate_okf_document(res["markdown"])
        self.assertTrue(doc["valid"])


if __name__ == "__main__":
    unittest.main()
