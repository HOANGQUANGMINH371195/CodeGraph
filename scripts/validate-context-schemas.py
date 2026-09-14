#!/usr/bin/env python3
"""Validate versioned context contracts independently of the Rust crates.

This is a development/golden-fixture gate. It deliberately uses Python's
jsonschema implementation rather than deserializing through graph-protocol,
so schema drift cannot be hidden by sharing the same validator implementation.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator
from referencing import Registry, Resource


ROOT = Path(__file__).resolve().parents[1]
SCHEMA_DIR = ROOT / "schemas"
FIXTURE_DIR = ROOT / "fixtures"


def load_json(path: Path) -> Any:
    with path.open(encoding="utf-8") as stream:
        return json.load(stream)


def validator(path: Path, schema_dir: Path) -> Draft202012Validator:
    schema = load_json(path)
    envelope_path = schema_dir / "context-envelope.schema.json"
    envelope_schema = load_json(envelope_path)
    Draft202012Validator.check_schema(schema)
    registry = (
        Registry()
        .with_resource(schema["$id"], Resource.from_contents(schema))
        .with_resource(envelope_schema["$id"], Resource.from_contents(envelope_schema))
    )
    return Draft202012Validator(schema, registry=registry)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def check_context_semantics(document: dict[str, Any]) -> None:
    """Check relations JSON Schema intentionally cannot express."""

    project = document["project"]
    graph_version = document["graph_version"]
    evidence = {item["id"]: item for item in document["evidence"]}
    require(len(evidence) == len(document["evidence"]), "duplicate evidence id")
    for item in document["evidence"]:
        require(item["project"] == project, "evidence crosses project scope")
        require(item["graph_version"] == graph_version, "evidence crosses graph scope")
        require(item["end_line"] >= item["start_line"], "evidence range is inverted")

    node_ids = {item["id"] for item in document["nodes"]}
    require(len(node_ids) == len(document["nodes"]), "duplicate node id")
    for item in document["nodes"]:
        if item["source_derived"]:
            require(item["evidence_ids"], "source-derived node lacks evidence")
        if item["resolution"] == "resolved":
            require(item["evidence_ids"], "resolved node lacks evidence or explicit uncertainty")
        require(
            set(item["evidence_ids"]).issubset(evidence),
            "node references missing evidence",
        )

    for item in document["edges"]:
        require(item["from"] in node_ids and item["to"] in node_ids, "dangling edge")
        if item["source_derived"]:
            require(item["evidence_ids"], "source-derived edge lacks evidence")
        if item["resolution"] == "resolved":
            require(item["evidence_ids"], "resolved edge lacks evidence or explicit uncertainty")
        require(
            set(item["evidence_ids"]).issubset(evidence),
            "edge references missing evidence",
        )

    for item in document["code_slices"]:
        require(item["end"] >= item["start"], "slice range is inverted")
        citations = [evidence[eid] for eid in item["evidence_ids"]]
        require(
            any(
                citation["path"] == item["path"]
                and citation["start_line"] <= item["start"]
                and citation["end_line"] >= item["end"]
                for citation in citations
            ),
            "slice is not covered by same-path evidence",
        )

    budget = document["budget"]
    for emitted, maximum in (
        ("emitted_nodes", "max_nodes"),
        ("emitted_edges", "max_edges"),
        ("emitted_source_ranges", "max_source_ranges"),
        ("emitted_source_bytes", "max_source_bytes"),
        ("emitted_serialized_bytes", "max_serialized_bytes"),
        ("emitted_characters", "max_characters"),
    ):
        require(budget[emitted] <= budget[maximum], f"{emitted} exceeds {maximum}")
    token_fields = (budget["max_tokens"], budget["tokenizer"], budget["emitted_tokens"])
    require(
        all(value is None for value in token_fields)
        or all(value is not None for value in token_fields),
        "tokenizer metadata is incomplete",
    )
    if token_fields[0] is not None:
        require(token_fields[2] <= token_fields[0], "emitted tokens exceed max tokens")


def check_pack_semantics(document: dict[str, Any]) -> None:
    envelope = document["envelope"]
    check_context_semantics(envelope)
    scope = document["scope"]
    require(scope["project"] == envelope["project"], "pack scope crosses project")
    require(scope["graph_version"] == envelope["graph_version"], "pack scope crosses graph")
    grant_scope = document["grant"]["scope"]
    require(grant_scope == scope, "grant widens pack scope")
    if document["handoff"] is not None:
        handoff = document["handoff"]
        require(handoff["project"] == scope["project"], "handoff crosses project")
        require(handoff["graph_version"] == scope["graph_version"], "handoff crosses graph")
        require(handoff["scope"] == scope, "handoff widens pack scope")
        claims = handoff["claims"]
        require(claims["project"] == scope["project"], "claims cross project")
        require(claims["graph_version"] == scope["graph_version"], "claims cross graph")
        require(claims["scope"] == scope, "claims widen handoff scope")
        for claim in claims["verified"] + claims["heuristic"]:
            require(claim["claim_state"] == "candidate", "accepted claim crossed pack boundary")
            require(claim["decision"] is None, "decided claim crossed pack boundary")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT, help="repository root")
    args = parser.parse_args()
    root = args.root.resolve()
    schema_dir = root / "schemas"
    fixture_dir = root / "fixtures"

    checks = (
        (schema_dir / "context-envelope.schema.json", fixture_dir / "context-envelope"),
        (schema_dir / "context-pack.schema.json", fixture_dir / "context-pack"),
    )
    passed = 0
    for schema_path, directory in checks:
        check = validator(schema_path, schema_dir)
        for fixture_path in sorted(directory.glob("*.json")):
            document = load_json(fixture_path)
            errors = list(check.iter_errors(document))
            invalid = fixture_path.name.startswith("invalid-")
            if schema_path.name == "context-envelope.schema.json":
                semantic_check = check_context_semantics
            else:
                semantic_check = check_pack_semantics
            if errors:
                require(invalid, f"unexpected schema result: {fixture_path.name}")
            else:
                try:
                    semantic_check(document)
                except ValueError:
                    require(invalid, f"unexpected semantic result: {fixture_path.name}")
                else:
                    require(not invalid, f"expected invalid fixture: {fixture_path.name}")
            passed += 1
    print(f"validated {passed} context schema fixtures and 2 schemas")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"context schema validation failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
