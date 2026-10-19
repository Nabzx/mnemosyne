# Research: dependency fit of google-adk in the adapters CI venv (issue #576)

Feeds ADR-0026. A real venv, not a paper check: built the exact
sequence the `adapters` CI job runs - `uv venv --python 3.12`, then
the SDK and all four existing adapter packages in the job's own
order - then installed `google-adk` on top, exactly reproducing CI's
own `python-version: "3.12"` pin rather than whatever Python happens
to be on hand.

## The `mcp` pin is compatible for real, not just on paper

`google-adk[mcp]==2.12.0` and `mnem-mcp`'s own `mcp>=2.2,<3` both
resolve to a single shared `mcp==2.3.0` - checked both with and
without requesting `google-adk`'s own `mcp` extra, same result
either time, no conflict `uv`'s resolver had to break.

## Three shared dependencies get downgraded, with no observed breakage

Installing `google-adk==2.12.0` on top of the four existing adapters
downgrades three packages already resolved by them:

| Package | Before | After |
| --- | --- | --- |
| `opentelemetry-api` | 1.45.1 | 1.42.1 |
| `tenacity` | 9.2.1 | 9.1.4 |
| `websockets` | 16.1.1 | 15.0.1 |

Not a hard conflict - `uv` found one consistent resolution - but a
real version regression on three packages the existing adapters
depend on. Ran the real test suites afterwards rather than assuming
the downgrade was safe: `pytest packages/mnem-mcp packages/
mnem-langgraph packages/mnem-openai-agents packages/mnem-autogen`
still passes all 88 tests with every downgraded package in place.

## A real, confirmed incompatibility: the Python floor

`google-adk`'s own PyPI metadata declares `requires_python: ">=3.11"`.
Every `mnem-*` package in this repo, `mnem-adk` would be the first
exception, currently declares `requires-python = ">=3.10"`. Checked
directly, not just read off metadata: building a real Python 3.10
venv and running `uv pip install google-adk==2.12.0` fails outright:

```
No solution found when resolving dependencies:
  Because the current Python version (3.10.20) does not satisfy
  Python>=3.11 and google-adk==2.12.0 depends on Python>=3.11, we can
  conclude that google-adk==2.12.0 cannot be used.
```

**This means**: `packages/mnem-adk/pyproject.toml` needs its own
`requires-python = ">=3.11"`, one version higher than every other
adapter package. This is a real decision for ADR-0026, not a detail
to default past - a user on Python 3.10 can install every other
adapter here today and would hit a hard install failure on this one
alone.

## Windows: no red flags in the dependency tree, real CI run still owed

`google-adk` itself ships as a pure `py3-none-any` wheel - platform
irrelevant for the package itself. Checked the two packages in its
real resolved dependency tree with compiled extensions
(`uuid-utils`, `ormsgpack`): both publish real Windows wheels on
PyPI, not sdist-only. No package in the full resolved set (74
packages) showed a Windows-only gap. This is real evidence, not a
guess, but it is not the same as a real Windows install: #19's own
`windows-python` CI job is still the actual proof, and should run
before this finding is treated as settled.

## Recommendation for ADR-0026

1. The `mcp` pin compatibility claim is confirmed: no change needed
   to either package's own constraints.
2. Record the three dependency downgrades and the fact that all 88
   existing adapter tests still pass under them - a real, checked
   fact, not an assumption, worth keeping in case a future `google-adk`
   bump tightens these further.
3. `packages/mnem-adk/pyproject.toml` must declare
   `requires-python = ">=3.11"` - a real, necessary exception to this
   project's otherwise-uniform `>=3.10` floor across every adapter
   package. State this plainly in the ADR and in `mnem-adk`'s own
   README, rather than letting a Python 3.10 user discover it as an
   install failure.
4. Windows has no found red flags but is not yet proven; #19 is the
   ticket that actually proves it.

Related: #568 (map), #569 (the base contract this dependency check
assumes nothing about), #583 (ADR-0026), #587 (CI wiring, must set
Python 3.11+ for the `mnem-adk` job specifically, not reuse the
other adapters' 3.10 floor), #598 (the cross-adapter id collision
test, unaffected by this finding).
