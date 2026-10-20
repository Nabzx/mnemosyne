# mnem-adk

A Google ADK `BaseMemoryService` backed by [Mnemosyne](https://github.com/Nabzx/mnemosyne).

An independent, third-party ADK memory service, not built or endorsed by Google.

Scaffolding only (#586). Every `BaseMemoryService` method still raises
`NotImplementedError` until its own ticket lands (see ADR-0026). This
README gets its real content, known limits and a measured per-turn
overhead figure in #599.
