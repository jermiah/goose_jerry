Based on our conversation so far, could you create:

1. A concise set of instructions (1–2 paragraphs) that describe how to handle coding-related tasks. Make the instructions generic and higher-level so that they can be re-used across different programming problems. Pay special attention to required output styles (plan, changes, code, run/verify, next step) and language-specific guidance (Python, TypeScript, Rust). Note that testing and debugging patterns should always be included.

2. A list of 3–5 example activities (as a few words each) that would be relevant to coding workflows.


Format your response in _VALID_ JSON, with one key being `instructions` (string) and the other key `activities` (array of strings).

For example :

{
  "instructions": "For coding requests, always return: (1) a concise plan, (2) file changes with rationale, (3) complete code blocks, (4) exact run/test commands with expected signals, (5) the smallest next step. Apply language guardrails (Python/TS/Rust), add tests with the change, and follow the debugging pattern: show error → cause → smallest fix → re-run.",
  "activities": [
    "Implement function with tests",
    "Fix failing unit tests",
    "Refactor module safely",
    "Add lint/type checks",
    "Create minimal CI script"
  ]
}
