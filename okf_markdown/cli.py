from __future__ import annotations
import argparse, json, sys
from pathlib import Path
from . import convert_transcript_to_okf, validate_okf_document


def main() -> None:
    parser = argparse.ArgumentParser(prog="okf-py", description="OKF v0.2 Python Validator & VTT/SRT Converter")
    sub = parser.add_subparsers(dest="cmd")
    p_val = sub.add_parser("validate")
    p_val.add_argument("path")
    p_ing = sub.add_parser("ingest")
    p_ing.add_argument("input")
    p_ing.add_argument("--title", default="Recorded Meeting Session")
    args = parser.parse_args()
    if args.cmd == "validate":
        res = validate_okf_document(Path(args.path).read_text(encoding="utf-8"), args.path)
        print(json.dumps(res, indent=2))
        sys.exit(0 if res["valid"] else 1)
    if args.cmd == "ingest":
        res = convert_transcript_to_okf(Path(args.input).read_text(encoding="utf-8"), title=args.title)
        sys.stdout.write(res["markdown"])
        sys.exit(0)
    parser.print_help()


if __name__ == "__main__":
    main()
