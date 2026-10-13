# Play against an agent

A walkthrough for a first game between you, in the window, and an AI agent coaching the other team through the MCP server. The agent can be a Claude model, an open-weight model served by Scaleway, or a local model run by Ollama or oMLX.

## How it fits together

- **The window hosts the match.** When a seat is `agent`, it also opens an MCP endpoint on `http://127.0.0.1:7878/mcp`. Each seat has a secret token, and the server knows which team a request coaches from that token alone.
- **The agent is an MCP client holding the token.** Here the client is always Claude Code, started by `scripts/coach.sh` with the protocol prompt `prompts/coach.md` and nothing else: only the four game tools, no skills, none of this machine's memory. Changing the model only changes where Claude Code sends its requests, so games against different models stay comparable.
- **Everything is saved in `runs/`.** The match record (every command, replayable) and the log of every tool call, written at the final whistle and whenever the match is stopped. `make assess` reads both.

Anthropic supports Claude Code with Claude models only. The local and Scaleway routes below point it at Anthropic-compatible endpoints run by other software, outside that support.

## 1. Start the game

```bash
make run ARGS="--away agent --away-turn-seconds 300"
```

You play home (blue, west den) and the agent plays away (gold, east den). Swap with `--home agent --away human`.

- **Turn time.** 60 s per seat by default. Give a slow model more time with `--away-turn-seconds`: 300 s for a cloud model, 600 s or more for a local model on a laptop. Its clock waits up to 15 s for the agent to look at its turn before starting.
- **Odds.** `--odds-hints full` (default) shows the agent every chance; `dice` shows only which dice face off; `none` shows neither.
- **Same dice again.** `--seed 3` replays the dice of an earlier game, as far as the same orders are given.
- **Fixed token.** `GB_AWAY_TOKEN=my-secret make run ARGS="--away agent"` avoids copying a random one.

The console prints how to connect, for example:

```text
Goblins Bowl: agents coach through http://127.0.0.1:7878/mcp
  away seat: agent, connect Claude Code with
    claude mcp add --transport http goblinsbowl-away http://127.0.0.1:7878/mcp --header "Authorization: Bearer my-secret"
  or coach it headless with
    scripts/coach.sh away http://127.0.0.1:7878/mcp my-secret runs/window
  record: runs/1791640000-seed12345.jsonl
  tool calls: runs/1791640000-seed12345-calls.jsonl
```

Closing the window or pressing Ctrl-C in the terminal saves the record.

## 2. Connect the agent

Every route runs the same command in a second terminal, only the model and a few environment variables change:

```bash
scripts/coach.sh away http://127.0.0.1:7878/mcp my-secret runs/first-game --model <model>
```

The coach's transcript goes to `runs/first-game/away-transcript.jsonl`, and `away-setup.txt` keeps the prompt's hash and the flags. If the coach stops before the final whistle, resume it with its context: `GB_RESUME_SESSION=<session_id from the transcript> scripts/coach.sh ...` with the same arguments.

### Claude models

```bash
scripts/coach.sh away http://127.0.0.1:7878/mcp my-secret runs/first-game --model claude-haiku-5-5
```

This uses your Claude Code login, so your plan's session limit applies. In the assessment batches under rules 0.1 (10 turns per half on the 15 × 15 bowl), a whole match took Haiku 5.5 between 3 and 6 minutes, and Claude Code counted $4 to $6 of usage for it; rules 0.2 matches last 12 turns per half, so expect more. With `ANTHROPIC_API_KEY` set, the match is billed per token to that key instead. Model ids include `claude-haiku-4-5`, `claude-haiku-5-5`, `claude-sonnet-4-6`, `claude-sonnet-5` and `claude-sonnet-5-5`; the CLI warns `unrecognized_model` for the two 5.5 ids but uses them.

To watch the agent think in an interactive session instead, run the printed `claude mcp add` line, start `claude --allowedTools "mcp__goblinsbowl-away__*"` so the game tools run without asking you each time, and paste the text of `prompts/coach.md` with `TEAM_NAME` replaced by `away (A, east den)`. That session also sees your usual memory and skills, which is fine for a game but not for an assessment.

### Local models with Ollama

Ollama (v0.14 and later) answers Anthropic Messages requests itself.

```bash
OLLAMA_CONTEXT_LENGTH=64000 ollama serve      # in its own terminal
ollama pull qwen3-coder:30b                   # 19 GB; any model with tool support: ollama.com/search?c=tools

export ANTHROPIC_BASE_URL=http://localhost:11434
export ANTHROPIC_AUTH_TOKEN=ollama            # required but ignored; it replaces your Claude login in this shell
export ANTHROPIC_API_KEY=""
export ANTHROPIC_DEFAULT_HAIKU_MODEL=qwen3-coder:30b   # Claude Code's background tasks use this tier
export CLAUDE_CODE_MAX_CONTEXT_TOKENS=64000   # the window Ollama serves
export CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1
export CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS=1

scripts/coach.sh away http://127.0.0.1:7878/mcp my-secret runs/first-game-ollama --model qwen3-coder:30b
```

The context window is the setting to watch. Claude Code's instructions, the four game tools and the coach prompt take about 13,000 tokens before the first view, and over a whole match Haiku 5.5's context grew past 160,000 tokens before Claude Code compacted it. Ollama's default window is only 4,000 tokens on a machine with less than 24 GiB of GPU memory, and its documentation asks for at least 64,000 for agents. Give the same number to both sides: `OLLAMA_CONTEXT_LENGTH` for the server, `CLAUDE_CODE_MAX_CONTEXT_TOKENS` for Claude Code, which then summarises the conversation (with the same model) before it overflows. `ollama ps` should show the model at 100 % GPU; if it does not, lower the window or pick a smaller model.

### Local models with oMLX

oMLX serves MLX models on Apple Silicon and also answers Anthropic Messages requests.

```bash
brew tap jundot/omlx https://github.com/jundot/omlx
brew install jundot/omlx/omlx
omlx serve --model-dir ~/models --api-key local-secret   # MLX models, for example from huggingface.co/mlx-community

export ANTHROPIC_BASE_URL=http://localhost:8000          # oMLX's default port, OMLX_PORT changes it
export ANTHROPIC_AUTH_TOKEN=local-secret                 # if oMLX answers 401, use ANTHROPIC_API_KEY instead
export ANTHROPIC_DEFAULT_HAIKU_MODEL=<model name>
export CLAUDE_CODE_MAX_CONTEXT_TOKENS=64000              # what the model and your memory allow
export CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1
export CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS=1

scripts/coach.sh away http://127.0.0.1:7878/mcp my-secret runs/first-game-omlx --model <model name>
```

oMLX detects tool calling for the Qwen, Llama, GLM, Gemma, Mistral, MiniMax, Kimi K2 and DeepSeek families. This route has not been tried in this repository yet; check the endpoint first (next section).

### Open-weight models on Scaleway

Scaleway's Generative APIs speak OpenAI's format at `https://api.scaleway.ai/v1`, with your secret key as a bearer token. A translation proxy, LiteLLM, gives Claude Code the Anthropic format in front of it. Pin a LiteLLM version you have checked: releases 1.82.7 and 1.82.8 shipped credential-stealing malware.

```yaml
# litellm.yaml
model_list:
  - model_name: scw-qwen
    litellm_params:
      model: hosted_vllm/qwen3-235b-a22b-instruct-2507
      api_base: https://api.scaleway.ai/v1
      api_key: os.environ/SCW_SECRET_KEY
general_settings:
  master_key: sk-local-litellm
```

```bash
python3 -m venv ~/.venvs/litellm
~/.venvs/litellm/bin/pip install 'litellm[proxy]==<a version you checked>'
SCW_SECRET_KEY=<your key> ~/.venvs/litellm/bin/litellm --config litellm.yaml --port 4000

export ANTHROPIC_BASE_URL=http://localhost:4000
export ANTHROPIC_AUTH_TOKEN=sk-local-litellm
export ANTHROPIC_DEFAULT_HAIKU_MODEL=scw-qwen
export CLAUDE_CODE_MAX_CONTEXT_TOKENS=<the window Scaleway gives for the model>
export CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1
export CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS=1

scripts/coach.sh away http://127.0.0.1:7878/mcp my-secret runs/first-game-scaleway --model scw-qwen
```

Serverless models with tool calling, according to Scaleway's catalogue in October 2026: `qwen3-235b-a22b-instruct-2507`, `qwen3.5-397b-a17b`, `qwen3.6-35b-a3b`, `qwen3.8-27b`, `glm-5.2`, `deepseek-v4-flash-0731`, `mistral-small-3.2-24b-instruct-2506`, `mistral-medium-3.5-128b`, `devstral-2-123b-instruct-2512`, `llama-3.3-70b-instruct` and `gemma-4-26b-a4b-it`. `gpt-oss-120b` calls tools only through the Responses API, which this LiteLLM entry does not use. This route has not been tried in this repository yet.

### Check an endpoint before a game

With the variables exported, one request tells whether the endpoint and the credential work:

```bash
curl -sS -w '\n%{http_code}\n' -X POST "$ANTHROPIC_BASE_URL/v1/messages" \
  -H "Authorization: Bearer $ANTHROPIC_AUTH_TOKEN" \
  -H "anthropic-version: 2023-06-01" \
  -H "content-type: application/json" \
  -d '{"model": "<model>", "max_tokens": 16, "messages": [{"role": "user", "content": "Say OK"}]}'
```

A JSON message holding some text, followed by `200`, is good. A `401` means the credential sits in the wrong variable: switch between `ANTHROPIC_AUTH_TOKEN` and `ANTHROPIC_API_KEY`.

## 3. What the agent sees and does

The coach follows the loop in `prompts/coach.md`: read the rules once, wait for its turn, play its goblins with one or a few `play` calls, and wait again until the status is `game_over`.

| Tool | Arguments | Returns |
|---|---|---|
| `wait_for_my_turn` | `max_wait_seconds` (at most 50, default 45) | the view as soon as it is the agent's turn or the match is over, else when the wait runs out |
| `get_state` | none | the view now |
| `get_rules` | none | the rules, the English part of `docs/rules.md` |
| `play` | `activations`, `end_turn`, `dry_run`, `expected_version` | one report per order, the plan's chance for a dry run, then the view |
| `compare_plans` | `plans`, each a list of `orders` | for each plan, one report per order and the chance that the whole plan goes through |

Each entry of `activations` is an order: a goblin, where it goes with the steps it has left this turn, and optionally its action. Since rules 0.2 a goblin has its Move and one action per turn, spent over as many orders as the coach likes, so it can walk, act, and walk on later in the turn:

```json
{"activations": [
   {"goblin": "A6", "to": [12, 8], "action": {"type": "stance"}, "estimate": 1.0},
   {"goblin": "A2", "path": [[10, 10]], "action": {"type": "assault", "target": "H2"}, "estimate": 0.62},
   {"goblin": "A2", "to": [11, 10], "estimate": 1.0},
   {"goblin": "A9", "action": {"type": "throw", "target": "A5"}}
 ],
 "end_turn": true}
```

`to` walks the safest path; `path` lists the exact squares stepped into, without the starting one. Orders resolve in order and stop at the first that ends the turn (a knockdown, a fumble, a goal) or that the server refuses; otherwise the turn lasts until `end_turn: true` or its clock runs out, even once no goblin can play. `dry_run: true` previews the orders as one plan without rolling: each order from where the previous ones leave the board if their rolls succeed, with its risky steps (the dice and chances the hint level allows) and, under full hints, the chance that the whole plan goes through (`plan_chance`). `estimate` is the agent's own chance of success, kept in the call log to measure calibration.

`compare_plans` weighs alternatives in one call: each plan is previewed from the board as it stands now, its orders in sequence. "Walk or throw?" is two plans of one order each; "dodge then throw, or throw from here with the malus?" is a plan of two orders against a plan of one, two smaller risks against a bigger one:

```json
{"plans": [
   {"orders": [{"goblin": "A9", "to": [16, 8]}, {"goblin": "A9", "action": {"type": "throw", "target": "A5"}}]},
   {"orders": [{"goblin": "A9", "action": {"type": "throw", "target": "A5"}}]}
 ]}
```

The view carries the status (`your_turn`, `waiting`, `game_over`), the seat and its direction of attack, the version, the time left, score and turns, the ball, the board as text rows, every goblin (characteristics, health, square, condition, whether it can still act, its `steps_left` and `has_action_left`), the assault and ball options (pick-ups, scoring routes) with their chances under full hints, and the events since the agent's previous call. At kick-off it is about 10,000 characters.

The server refuses an order with a message the model reads and the match untouched: not your turn, a stale plan (`expected_version` behind the match), a turn that changed since the agent last looked, or an order the rules forbid, with the reason (a square beyond its steps left, a goblin with no step or action left, an action already used).

### Talk to the server yourself

The MCP revision used here (2026-07-28) needs no session: every request is a plain HTTP POST carrying the seat token, the method in a header, and the protocol metadata. This is a whole `get_state` call:

```bash
curl -s -X POST http://127.0.0.1:7878/mcp \
  -H "Authorization: Bearer my-secret" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -H "MCP-Protocol-Version: 2026-07-28" \
  -H "Mcp-Method: tools/call" \
  -H "Mcp-Name: get_state" \
  -d '{"jsonrpc": "2.0", "id": 1, "method": "tools/call",
       "params": {"name": "get_state", "arguments": {},
                  "_meta": {"io.modelcontextprotocol/protocolVersion": "2026-07-28",
                            "io.modelcontextprotocol/clientCapabilities": {}}}}'
```

The answer is one `data:` line holding a JSON-RPC result, whose `structuredContent` is the view (also given as text in `content[0].text`). `tools/list` works the same way with `Mcp-Method: tools/list`, no `Mcp-Name`, and no `name` or `arguments`. Any MCP client that can send a bearer header to a Streamable HTTP server can coach a seat.

## 4. After the game

`runs/` holds the match record and the tool calls, and the coach's folder holds its transcript and setup. How each side handled risk:

```bash
make assess ARGS="runs/1791640000-seed12345.jsonl"
```

For each team it prints the safe-first index (did certain orders come before gambles), failures and how many goblins were still ready when one ended a turn, the gap between the chance taken and the best chance for the same purpose, the chance taken right after a failed turn and when behind, level or ahead, the health lost to its own activations, the refused calls and the turns lost to the clock. When the agent sent estimates, a calibration line compares them with the outcomes and with the true odds. [`assessment.md`](assessment.md) defines each measure.

## 5. Assessment batches with another model

The batch scripts run any model the same way, with the variables of its route exported:

```bash
GB_PARALLEL=1 GB_TURN_SECONDS=300 GB_BUDGET=1000 scripts/assessment-batch.sh local-qwen qwen3-coder:30b
make assess ARGS="--batch runs/local-qwen"
```

It plays seeds 1 to 5 from each seat against the cautious bot, one match at a time on a local model. `GB_BUDGET` caps the cost Claude Code counts per coach run (10 USD by default, sized for a 24-turn match); a local model costs nothing, so lift the cap rather than let an estimate stop the coach. The report has three tables: the model's measures, the bot's measures in the same matches as a yardstick, and the pass or fail of each criterion fixed in [`assessment.md`](assessment.md). Results compare fairly between models run through the same client with the same turn time; note both next to any comparison.

## Troubleshooting

| What you see | Why | What to do |
|---|---|---|
| The agent loses its first turn | Its first decision, with an empty context, takes longer than the turn | Give its seat more time (`--away-turn-seconds`) |
| `the turn changed since you last looked` | The agent planned on an old view | Nothing: the next `get_state` brings it back in step |
| `401` from the game server | Wrong or missing seat token | Use the token the console printed, in `Authorization: Bearer` |
| Claude Code still uses your subscription, or asks to log in | `ANTHROPIC_AUTH_TOKEN` is not set in that terminal | Export it before `scripts/coach.sh` |
| `400` errors naming unknown fields from a local endpoint | Claude Code sends fields the endpoint does not know | `export CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS=1` |
| Claude Code ignores the variables you exported | An `env` block in `~/.claude/settings.json` overrides them | Remove those entries while you play |
| A local coach forgets the match or repeats old orders | Its conversation outgrew the model's window | Raise `OLLAMA_CONTEXT_LENGTH` and set `CLAUDE_CODE_MAX_CONTEXT_TOKENS` to the same value |
| `You've hit your session limit` | Your Claude plan's limit for the next hours is used up | Wait for the reset, or use an API key |
| The coach stops before the final whistle | The model ended its session early | Resume it with `GB_RESUME_SESSION` (section 2) |

## Sources

- [Ollama: Anthropic compatibility](https://docs.ollama.com/api/anthropic-compatibility), [Claude Code with Ollama](https://docs.ollama.com/integrations/claude-code) and [context length](https://docs.ollama.com/context-length)
- [oMLX](https://github.com/jundot/omlx)
- [Scaleway Generative APIs: supported models](https://www.scaleway.com/en/docs/generative-apis/reference-content/supported-models/) and [integrating with popular tools](https://www.scaleway.com/en/docs/generative-apis/reference-content/integrating-generative-apis-with-popular-tools/)
- [LiteLLM: OpenAI-compatible endpoints](https://docs.litellm.ai/docs/providers/openai_compatible) and [the Anthropic-format endpoint](https://docs.litellm.ai/docs/anthropic_unified)
- [Claude Code: connect to an LLM gateway](https://code.claude.com/docs/en/llm-gateway-connect), [other LLM gateways](https://code.claude.com/docs/en/llm-gateway), [the window of a custom model ID](https://code.claude.com/docs/en/model-config#correct-the-window-for-a-gateway-or-custom-model-id) and [environment variables](https://code.claude.com/docs/en/env-vars)
