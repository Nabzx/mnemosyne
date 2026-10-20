"""The search ranking locked in #573 (ADR-0026): matching
``InMemoryMemoryService``'s own real keyword-overlap algorithm exactly,
ported directly from the ``v2.12.0`` source, not approximated.

NFC-normalised ``\\w+`` tokenising, Latin/non-Latin script-run widening (so
a CJK sentence embedding a Latin word still exposes that word as its own
token), a non-ASCII query token also matching by plain substring over the
lowered text (CJK carries no word boundaries for token-set membership
alone to catch), zero-score entries dropped, stable descending sort by
score, top 10.
"""

from __future__ import annotations

import itertools
import re
import unicodedata

_MAX_SEARCH_RESULTS = 10


def _is_latin(c: str) -> bool:
    if c.isascii():
        return c.isalnum() or c == "_"
    return unicodedata.name(c, "").startswith("LATIN")


def extract_words_lower(text: str) -> set[str]:
    """Unicode-aware tokens from ``text``, lowercase."""
    text = unicodedata.normalize("NFC", text)
    return {word.lower() for word in re.findall(r"\w+", text)}


def extract_searchable_words(text: str) -> set[str]:
    """The tokens of :func:`extract_words_lower`, plus, for a token mixing
    Latin and non-Latin scripts, each of its single-script runs."""
    words = extract_words_lower(text)
    for word in list(words):
        if not word.isascii():
            for _is_latin_run, group in itertools.groupby(word, _is_latin):
                words.add("".join(group))
    return words


def text_from_content_dict(content: dict) -> str:
    """The plain text of a stored, serialised ``Content`` dict - every
    ``parts[*].text``, joined, matching how the reference service reads a
    live ``Content`` object's own text parts."""
    parts = content.get("parts") or []
    return " ".join(part["text"] for part in parts if part.get("text"))


def score(query: str, text: str) -> int:
    """How many of ``query``'s own words ``text`` contains - 0 if none,
    the same matching rule the reference service uses: an ASCII query word
    must be in the text's own token set; a non-ASCII one may also match by
    plain substring, since CJK text carries no word boundaries."""
    words_in_query = extract_words_lower(query)
    words_in_text = extract_searchable_words(text)
    if not words_in_text:
        return 0

    text_lower = unicodedata.normalize("NFC", text).lower()
    return sum(
        1
        for query_word in words_in_query
        if query_word in words_in_text or (not query_word.isascii() and query_word in text_lower)
    )


def top_matches(query: str, candidates: list[tuple[str, str]]) -> list[str]:
    """The ids of the top (at most 10) ``candidates`` - each a
    ``(id, text)`` pair - ranked by :func:`score`, descending, ties kept
    in their given order (stable sort, the sort key reads only the
    score)."""
    scored = [(score(query, text), node_id) for node_id, text in candidates]
    scored = [(s, node_id) for s, node_id in scored if s > 0]
    scored.sort(key=lambda pair: -pair[0])
    return [node_id for _score, node_id in scored[:_MAX_SEARCH_RESULTS]]
