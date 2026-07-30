# id: sticky-attention-agent-session-lifecycle
# type: feature
# stories: specs/stories/sticky-attention/bind-agent-session-to-context.md, specs/stories/sticky-attention/record-agent-session-lifecycle.md, specs/stories/sticky-attention/checkpoint-durable-context.md
# journey: specs/journeys/maintain-a-durable-context-through-agent-sessions.md
# model: specs/models/sticky-attention/bounded-attention-context.model.yaml, specs/models/sticky-attention/agent-session.lifecycle.yaml
# contract: specs/contracts/agent-harness/durable-context-hooks.yaml, POST /events

@sticky-attention @agent-session @adapter-conformance
Feature: Maintain a durable context through portable agent-session hooks
  As a sovereign builder
  I want harness sessions to bind, leave lifecycle residue, and checkpoint
  context artifacts through a portable contract
  So that continuity survives harness changes and interrupted exits without
  expanding the disclosure or publication boundary

  Background:
    Given context "portable-relay" has an h boundary and context-owned hook opt-in
    And a generic harness adapter contains no context identifier or context-specific path
    And the local sovereign relay is available

  @binding @context-policy
  Scenario: Honor context-owned integration policy
    Given the context opt-in names its identity, hooks, disclosure policy, and checkpoint limits
    And the adapter presents a conflicting context identifier
    When the adapter resolves the session binding
    Then the context-owned identifier remains authoritative
    And the adapter override is rejected
    And no lifecycle residue is written

  @binding @context-root
  Scenario: Bind a session opened in the context root
    Given the harness working directory is inside the enabled context root
    When the adapter resolves the session binding
    Then exactly one binding names context "portable-relay"
    And the binding records the matched root role

  @binding @linked-repository
  Scenario: Bind a session opened in a declared linked repository
    Given repository "buzz" is a linked directory declared by context "portable-relay"
    And the harness working directory is inside repository "buzz"
    When the adapter resolves the session binding
    Then exactly one binding names context "portable-relay"
    And the binding records the linked-repository role

  @binding @no-match
  Scenario: Leave a session unbound when no context claims it
    Given the harness working directory is outside every enabled context
    When the adapter resolves the session binding
    Then it returns a no-context-match result
    And no lifecycle residue is written

  @binding @ambiguity
  Scenario: Fail closed when multiple contexts claim the working directory
    Given two enabled contexts declare the same linked repository
    When the adapter resolves the session binding
    Then it returns an ambiguous-context result
    And no lifecycle residue is written to either context

  @binding @portability
  Scenario: Reuse one adapter across independent contexts
    Given a second context owns a different identity and hook opt-in
    When sessions start in each context
    Then the unchanged adapter resolves each context from its own policy
    And neither session can write lifecycle residue across the other boundary

  @binding @identity
  Scenario: Namespace harness-local session identifiers
    Given two harness adapters each report session ID "42" in context "portable-relay"
    When both sessions are bound
    Then their identities remain distinct by adapter ID
    And neither adapter can complete or interrupt the other session

  @lifecycle @policy
  Scenario: Do not write a disabled lifecycle hook
    Given the harness session is uniquely bound
    And context policy disables the session-start hook
    When the start callback arrives
    Then the adapter returns hook-disabled
    And no lifecycle residue is written

  @lifecycle @start @privacy
  Scenario: Record metadata-only session start
    Given the harness session is uniquely bound
    When the start callback arrives
    Then a signed start record carries the context boundary
    And it contains the context ID, harness session ID, adapter ID, phase, and timestamp
    And optional working-directory metadata satisfies context disclosure policy
    And it contains no prompt, response, transcript, credential, error body, or final message

  @lifecycle @privacy @redaction
  Scenario: Omit lifecycle metadata disallowed by context policy
    Given the harness session is uniquely bound
    And context disclosure policy disallows working-directory paths
    When the start callback arrives
    Then the signed start record omits the working directory
    And all required lifecycle metadata remains present

  @lifecycle @completion
  Scenario: Record normal session completion
    Given a bound session has an active start record
    When its end callback arrives with a bounded completion-reason code
    Then a signed completion record terminates that session
    And the session reads as completed

  @lifecycle @completion @failure
  Scenario: Surface an end-write failure without fabricating completion
    Given a bound session has an active start record
    And the local relay rejects its completion record
    When the end callback returns
    Then the adapter reports lifecycle-write-failed
    And the session remains active for later reconciliation

  @lifecycle @interruption @reconciliation
  Scenario: Reconcile a missing end callback on the next start
    Given an older bound session remains active
    And no completion record exists
    And the adapter confirms the older host process is no longer live
    When a later bound session starts in the same context
    Then an interruption record marks the older session interrupted first
    And the later session start is recorded afterward
    And repeating reconciliation creates no additional interruption record

  @lifecycle @concurrency
  Scenario: Do not interrupt a concurrent session
    Given an older bound session remains active
    And the adapter has no evidence that the older session was abandoned
    When a later bound session starts in the same context
    Then the older session remains active and visible
    And the later session receives an independent namespaced identity

  @lifecycle @delayed-completion
  Scenario: Correct interruption with a provably delayed completion
    Given a session was reconciled as interrupted
    And its delayed completion proves it ended before reconciliation
    When the completion record arrives
    Then the effective session state becomes completed
    And the interruption evidence remains in history

  @status @read-only
  Scenario: Inspect context drift without changing durable state
    Given root artifacts differ from the current manifest
    When the builder requests context status
    Then changed, removed, and untracked artifacts are reported
    And no artifact, tombstone, lifecycle, or manifest event is written

  @checkpoint @manifest-last
  Scenario: Checkpoint accepted artifact drift before replacing the manifest
    Given changed root artifacts pass context-owned policy
    And one previously manifested artifact was removed
    When the builder requests a checkpoint
    Then changed artifact events are durable
    And a removal tombstone is durable
    And the manifest is atomically replaced after those records
    And the manifest journal record is written last

  @checkpoint @failure
  Scenario: Keep the previous manifest when an artifact write fails
    Given changed root artifacts pass context-owned policy
    And one accepted artifact event cannot be made durable
    When the builder requests a checkpoint
    Then the checkpoint fails explicitly
    And the manifest is not replaced
    And the previous manifest remains authoritative

  @checkpoint @repository-boundary
  Scenario: Snapshot linked repositories without inventorying their contents
    Given the context root links to repository "buzz"
    When the builder requests a checkpoint
    Then inventory does not follow the symbolic link
    And repository "buzz" contributes only sanitized remote, branch, and commit metadata
    And embedded remote credentials are excluded
    And no repository working-tree file becomes a context artifact

  @checkpoint @safety
  Scenario Outline: Reject unsafe checkpoint candidates explicitly
    Given a candidate root artifact is <candidate>
    When the builder requests a checkpoint
    Then the candidate is rejected with <finding>
    And the previous manifest remains authoritative

    Examples:
      | candidate                                  | finding           |
      | a sensitive filename                       | sensitive-file    |
      | text matching a sensitive-content pattern | sensitive-content |
      | binary content                             | binary-file       |
      | larger than the size limit                 | oversized-file    |

  @publication @local-only
  Scenario Outline: Local context operations never publish
    When the adapter performs <operation>
    Then no replication, synchronization, or publication command is invoked

    Examples:
      | operation                   |
      | session start               |
      | session completion          |
      | interruption reconciliation |
      | status inspection           |
      | context checkpoint          |
