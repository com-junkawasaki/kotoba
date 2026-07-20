# Kotoba Application Profile

Kotoba is a safe application language for writing host orchestration, UI, LLM
workflows, state machines, and actor behavior in `.kotoba`. Broader application
semantics do not weaken T1 memory safety, T2 effect soundness, or T3 capability
confinement.

## Profiles

| Profile | Purpose | Effects |
|---|---|---|
| `kotoba/pure` | Deterministic computation | None |
| `kotoba/cell` | Bounded sandboxed computation | Explicit capability calls |
| `kotoba/app` | UI, workflow, LLM, state, and actor behavior | Typed effect commands |
| `kotoba/host` | Trusted runtime and capability providers | Policy-enforced mechanisms |

`kotoba/app` is not unrestricted Clojure or ClojureScript. It has no ambient
DOM, filesystem, network, credential, provider-SDK, reflection, dynamic-load,
`eval`, or process-global mutation access.

## State and effect model

An application transition consumes state and an event, then returns the next
state and bounded effect commands:

```clojure
(defn update [state event]
  {:state (next-state state event)
   :effects [{:cap :state/put :value persisted-view}
             {:cap :ui/render :value view-tree}]})
```

The host checks each effect against declared and inferred effects, supplied
capability values, package lock, policy, quota, fuel, memory, and audit rules.
Provider results return as typed events.

## Ownership boundary

Application code owns lifecycle decisions, workflows, view trees, reducers,
prompts, tool selection, LLM-result validation, state transitions, actor
behavior, and supervision policy.

Providers own Wasm bootstrap, DOM/WebGPU/native mutation, transport,
credentials, model SDK calls, durable storage, transactions, queues, timers,
placement, retry, and recovery. This confines external mechanisms without
moving product semantics out of Kotoba.

## Completion gate

A UI, LLM, state, actor, or lifecycle capability family is implemented only
when all of these land:

1. Typed descriptors in the closed capability contract.
2. Effect inference and compiler admission.
3. A policy-gated provider with no ambient fallback.
4. Positive and denial conformance fixtures.
5. Quota, cancellation, fuel/memory, and audit behavior where applicable.
6. Parity evidence across at least two applicable runtimes.

Documentation can mark a family planned before this gate passes, but must not
claim runtime support.

## LLM rule

LLM output is untrusted input. It may propose state or effects, but schema
validation and an application governor must accept the proposal before any
consequential capability runs. Models never receive credentials or an unscoped
provider client.

## Migration

The former narrow-slice-only ADR is removed. Equivalent narrow-slice and
general-application exclusions in ADR-2607150000 and ADR-2607151500 are
superseded and must not be used as an application-scope ceiling. Their
measurements describe the relevant legacy emitter/backend only.
Migrate complete vertical slices after their capabilities pass the completion
gate; do not mechanically rename `.cljc`, admit arbitrary dependencies, or
rewrite a whole repository without working parity.

The first proving slice is shiropico:

```text
state transition
  -> LLM or ComfyUI request effect
  -> typed result event
  -> governor validation
  -> UI render effect
  -> durable checkpoint
```

The authoritative workspace decision is ADR-2607201300.
