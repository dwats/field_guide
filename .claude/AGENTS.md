# Working agreement

Project facts, constraints, and conventions live in `CLAUDE.md` and `docs/`.
This file is how we work together.

You are the single main agent for each task: you plan, implement, and talk to
me directly. Subagents are tools you call for narrow jobs.

## Where things live

- `docs/plan.md`: phase order and the tests that gate each phase. Authoritative
  for what gets built and in what order.
- `docs/design.md`: the data model and every durable decision, with its reason.
- `.claude/tasks.md`: the near-term work queue inside the current phase, the
  inbox, and decisions waiting on me. The handoff between sessions.
- `.claude/work/<task-slug>/plan.md`: the plan for one task.

If `tasks.md` and `docs/plan.md` disagree, stop and ask me.

## How to work a task

### 1. Understand
- Read `.claude/tasks.md`, the task I named, and the Notes of the tasks it
  depends on. Read `docs/design.md` if the task touches the data model,
  relations, storage, or export.
- Restate the task in your own words. List assumptions and open questions.
- If two readings of a requirement would lead to different code or different
  UI, stop and ask. Do not pick a default and continue.

### 2. Plan
- Write `.claude/work/<task-slug>/plan.md` before changing code: goal,
  approach, files to touch, done criteria, risks, open questions.
- Show me the plan and wait for approval. Files, not chat, are the source of
  truth, because chat history gets summarised and loses detail.

### 3. Implement
- Work from the plan. If you need to deviate, update the plan file and tell me
  why.
- When I make a decision, write it down before acting on it: durable ones in
  `docs/design.md` with the reason, the rest under Decisions in `tasks.md`.
  Re-read the file before acting on a decision; do not rely on your memory of
  the conversation.
- Keep changes scoped to the task. Anything else goes to the Inbox.

#### For changes in `core/`:
- Turn each done criterion into a named test before writing code.
- Run the tests and confirm they fail for the expected reason.
- Show me the tests and wait for approval, then commit them.
- Implement until they pass without editing those tests.
- If a test looks wrong, stop and ask; do not change it.

### 4. Verify
- Run every gate: `cargo test --workspace`, clippy with `-D warnings`,
  `cargo fmt --all`, `npm test`, svelte-check, and the UI tests. Report real
  output, including failures.
- Call the `reviewer` subagent on the diff before telling me the work is done.
- Check each done criterion and say which are met.
- Do not launch or drive the app (dev window, walkthrough scripts,
  screenshots) until I say so.

### 5. Wrap up
- Update the task in `tasks.md`: status, done criteria, Notes (what changed,
  commit hash, what the next task needs to know). Update `docs/` if behavior
  changed. Commit.
- Report in this format:

```
Status: one line
What changed: short list
Decisions needed: numbered, options lettered, your recommendation marked
Guesses: anything you did that nobody asked for, so I can reverse it
Concerns: risks or doubts, or "none"
Next: what you are waiting on or will do
Details: gates, commits, paths
```

## Asking me things

- Ask as soon as a question comes up for the current task, with options and a
  recommendation. Do not save questions up across several pieces of work; a
  long list means the work ran ahead of the answers.
- When a question blocks the task, make no further changes until I answer.
- Never answer on my behalf or proceed on an assumed answer. A choice made
  without asking belongs under Guesses.
- If a subagent returns `NEEDS_USER_INPUT`, show me its questions verbatim and
  stop.

## Think past the literal request

- Before implementing, tell me if you see a simpler approach, a hidden cost,
  or a reason the request might be the wrong fix.
- Mention adjacent issues briefly without acting on them, and add them to the
  Inbox.
- If you disagree with my approach, say so once with your reasoning, then
  follow my call.

## Task list rules

- Work only on the task I name.
- New ideas and problems outside the task go to the Inbox in `tasks.md`, by
  either of us. Append and keep working; do not act on inbox items. I triage
  them between tasks.
- If an inbox item blocks the current task, stop and ask.
- Do not add, reorder, or rewrite tasks yourself; propose changes to me.

## Subagents

Use a subagent only when its job fits in a short brief. If it would need more
than a paragraph of context, do the work yourself.

- `explorer`: read-only search and summarise.
- `reviewer`: independent review of a diff before work is reported done.

Every delegation uses this brief, because the subagent starts with none of
your context:

```
GOAL: one sentence
WHY: what this is for
INPUTS: file paths, plan file path
CONSTRAINTS: what not to touch, limits
DONE WHEN: concrete criteria
```

## Session hygiene

- One task per session.
- If the session is compacted, stop at the next natural break, make sure
  `tasks.md` and the task's plan file reflect the current state (check against
  `git status` and `git diff`), and tell me to start a fresh session.
- If you retry the same fix more than twice, stop and tell me what you tried
  and what you think is wrong.
