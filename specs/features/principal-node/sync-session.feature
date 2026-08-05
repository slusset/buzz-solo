# id: principal-node-sync-session
# type: feature
# issue: https://github.com/slusset/buzz-solo/issues/8
# story: specs/stories/principal-node/place-synchronization-inside-principal-node.md
# journey: specs/journeys/evolve-a-principal-node.md
# model: specs/models/principal-node/sync-session.model.yaml
# lifecycle: specs/models/principal-node/sync-session.lifecycle.yaml
# contract: application-port PrincipalNodeSyncSession.request_sync

@principal-node @synchronization @issue-8
Feature: Run synchronization as one Principal-Node-owned procedure
  As a domain architect
  I want every synchronization attempt to use one guarded application procedure
  So that triggers, transports, and Runtime Instances cannot redefine continuity

  Background:
    Given a stable Principal Node represents exactly one Principal Domain
    And current authority, declarations, replication, transport, cursor, summary, identity, and clock capabilities are available only through inward-owned ports
    And SyncSession identity issuance and attempt clock sampling are infallible application prerequisites
    And the portable relay core owns deterministic event verification and sink decisions but no scheduling or network orchestration

  @request @prerequisite @validation
  Scenario Outline: Fail before exposing a lifecycle when prerequisites are absent
    Given the application composition cannot establish <missing-prerequisite>
    When a synchronization request reaches the composition boundary
    Then construction is refused as "application_prerequisite_unavailable"
    And no SyncSession identity or lifecycle state is exposed

    Examples:
      | missing-prerequisite       |
      | an infallible ID issuer    |
      | an infallible attempt clock |

  @request @trigger
  Scenario Outline: Route every trigger through one procedure
    Given <prior-session-evidence>
    When trigger "<trigger>" requests <direction> synchronization for source stream "shared/clinical-review-v1"
    Then the request enters the "request_sync" application operation
    And a compiler-distinct SyncSession identity is issued by the Principal Node boundary
    And transition "begin_evaluation" moves the new session from "requested" to "evaluating"
    And no trigger-specific synchronization lifecycle is selected

    Examples:
      | trigger          | direction | prior-session-evidence                       |
      | startup          | pull      | no prior session                             |
      | journal_commit   | push      | no prior session                             |
      | peer_wake        | pull      | no prior session                             |
      | recovery_tick    | pull      | no prior session                             |
      | operator_request | push      | no prior session                             |
      | retry            | pull      | an immutable failed prior session is present |

  @request @authority @validation
  Scenario Outline: Refuse authority-bearing trigger data
    Given an otherwise valid synchronization request
    When its trigger data attempts to supply trusted <forbidden-field>
    Then the request is rejected as "untrusted_trigger_authority"
    And no SyncSession is created
    And no authority, transport, replication, cursor, or summary port is called

    Examples:
      | forbidden-field    |
      | agreement          |
      | event selection    |
      | transport principal |
      | source cursor      |

  @evaluation @freshness @authority
  Scenario: Re-evaluate current authority and agreement for every attempt
    Given a completed attempt previously used agreement snapshot "agreement-head-a"
    And the current declaration projection now reports "agreement-head-a" as superseded
    When a new attempt begins for the same source stream and direction
    Then the clock bounds new evaluation evidence for this attempt
    And authenticated peer evidence is obtained anew
    And current domain and node authorization is evaluated anew
    And current declaration heads and selection are projected anew
    And "agreement-head-a" is not reused as trusted current authority

  @evaluation @blocked
  Scenario Outline: Block transfer when current evidence is not ready
    Given evaluation reports <not-ready-evidence>
    When the Principal Node evaluates the requested session
    Then one immutable blocked terminal summary identifying "<classification>" is durably committed
    And transition "refuse_transfer" exposes the session moving from "evaluating" to "blocked"
    And neither replication source nor replication sink transfers a record
    And no cursor commit is attempted

    Examples:
      | not-ready-evidence                          | classification             |
      | no current Principal Node authorization    | node_unauthorized           |
      | no matched current stream agreement        | agreement_not_ready         |
      | no authenticated admitted peer             | transport_not_ready         |
      | a required mechanical capability is absent | required_capability_missing |

  @evaluation @failure
  Scenario Outline: Fail evaluation with granular operational evidence
    Given a requested session has entered "evaluating"
    When evaluation encounters "<failure>"
    Then one immutable failed terminal summary identifying "<classification>" and bounded failure evidence is durably committed without a cursor mutation
    And transition "evaluation_failure" exposes the session moving from "evaluating" to "failed"

    Examples:
      | failure                       | classification               |
      | durable cursor load fails     | cursor_load_failed            |
      | peer transport is unavailable | transport_unavailable         |
      | current projection fails      | projection_unavailable        |
      | replication source fails      | source_unavailable            |
      | source batch is malformed     | malformed_source_batch        |

  @transfer @direction @exact-envelope
  Scenario Outline: Compose push and pull around the same exact-envelope procedure
    Given current authority, agreement, selection, and transport evidence are ready
    And the source returns a bounded batch of exact signed event envelopes
    When the session transfers in the "<direction>" direction
    Then transition "begin_transfer" moves the session from "evaluating" to "transferring"
    And source, sink, and authenticated transport adapters are composed for "<direction>"
    And each event ID, author, created_at, kind, tags, content, and signature reaches the sink unchanged
    And no adapter becomes event author or synchronization policy owner

    Examples:
      | direction |
      | pull      |
      | push      |

  @caught-up @zero-records @happy-path
  Scenario: Complete an empty caught-up page without cursor progress
    Given current authority, agreement, selection, and transport evidence are ready
    And the replication source reports no records after the durable source-bound cursor
    And the source classifies next_cursor as unchanged or equivalent to the agreed initial no-position
    And the source reports caught_up as true
    When the Principal Node evaluates the batch
    Then one immutable completed terminal summary classifying the attempt as "caught_up" is durably committed without a cursor mutation
    And transition "no_work_needed" exposes the session moving from "evaluating" to "completed"
    And records examined is 0
    And cursor committed is false
    And the required source-issued next_cursor remains batch output only
    And no durable cursor transition or receipt evidence is manufactured

  @scan-progress @zero-records @checkpoint
  Scenario Outline: Commit filtered scan progress without manufacturing receipts
    Given the immutable SourceStreamId predicate excluded every record in one bounded source page
    And the source classifies next_cursor as advanced from cursor_before
    And the source reports caught_up as <caught-up>
    When the Principal Node evaluates the empty page
    Then transition "begin_cursor_commit" moves the session from "evaluating" to "committing_cursor"
    And no sink call or receipt evidence is manufactured
    And Principal Node continuity atomically commits the exact next_cursor with a completed "<outcome>" summary containing records_examined 0
    And transition "cursor_durable" exposes the session moving from "committing_cursor" to "completed"

    Examples:
      | caught-up | outcome       |
      | false     | scan_progress |
      | true      | caught_up     |

  @scan-progress @zero-records @failure
  Scenario: Fail an empty non-caught-up page without cursor progress
    Given one bounded source page contains no selected records
    And the source reports caught_up as false
    And the source classifies next_cursor as unchanged from cursor_before
    When the Principal Node validates the page
    Then one immutable failed terminal summary identifying "malformed_source_batch" is durably committed without a cursor mutation
    And transition "evaluation_failure" exposes the session moving from "evaluating" to "failed"
    And no sink call or receipt evidence is manufactured

  @receipt @checkpoint @happy-path
  Scenario Outline: Commit the exact source cursor after checkpoint-safe receipts
    Given a bounded batch has been transferred with candidate cursor "opaque:batch-002"
    And every covered record has durable outcome "<outcome>"
    When the sink returns receipts bound to each event and source cursor
    Then transition "batch_delivered" moves the session from "transferring" to "awaiting_durable_receipts"
    And outcome "<outcome>" is checkpoint-safe
    And transition "receipts_checkpoint_safe" moves the session from "awaiting_durable_receipts" to "committing_cursor"
    And Principal Node continuity compares against the exact prior source-bound cursor
    And Principal Node continuity atomically persists "opaque:batch-002" unchanged with receipt evidence and one immutable completed terminal summary
    And transition "cursor_durable" exposes the session moving from "committing_cursor" to "completed"

    Examples:
      | outcome    |
      | stored     |
      | duplicate  |
      | superseded |

  @receipt @checkpoint @failure
  Scenario Outline: Fail closed when a receipt is not checkpoint-safe
    Given a bounded batch has reached "awaiting_durable_receipts"
    And at least one covered record has outcome "<outcome>"
    When the Principal Node classifies all receipts
    Then outcome "<outcome>" is not checkpoint-safe
    And one immutable failed terminal summary identifying "<classification>" is durably committed without a cursor mutation
    And transition "receipts_not_checkpoint_safe" exposes the session moving from "awaiting_durable_receipts" to "failed"
    And the durable cursor remains exactly its prior value
    And the batch remains eligible for a new retry session

    Examples:
      | outcome   | classification       |
      | rejected  | receipt_rejected     |
      | ambiguous | receipt_ambiguous    |

  @transport @failure
  Scenario: Fail without a cursor advance when transport fails
    Given a session is in "transferring"
    When authenticated peer transport cannot complete the bounded batch
    Then one immutable failed terminal summary identifying "transport_failed" is durably committed without a cursor mutation
    And transition "transport_failure" exposes the session moving from "transferring" to "failed"
    And the durable cursor remains exactly its prior value

  @cursor @failure
  Scenario: Fail without a partial advance when durable cursor commit fails
    Given a session is in "committing_cursor"
    And all covered destination receipts are checkpoint-safe
    When continuity definitively rejects compare-and-commit without storing the candidate cursor
    Then one immutable failed terminal summary identifying "cursor_commit_failed" is durably committed without a cursor mutation
    And transition "cursor_write_failed" exposes the session moving from "committing_cursor" to "failed"
    And the durable cursor remains exactly its prior value

  @continuity @persistence @retry
  Scenario Outline: Return exact pending continuity when persistence does not resolve
    Given a session is ready to commit "<terminal-state>" from "<previous-state>"
    When Principal Node continuity reports "<persistence-result>"
    Then neither terminal state nor terminal summary is exposed
    And the session remains in "<previous-state>"
    And a typed pending-continuity result contains the same SyncSession identity and exact immutable candidate commit
    And no retry SyncSession is created

    Examples:
      | terminal-state | previous-state               | persistence-result |
      | completed      | evaluating                   | unavailable        |
      | completed      | committing_cursor            | ambiguous          |
      | blocked        | evaluating                   | unavailable        |
      | failed         | evaluating                   | ambiguous          |
      | failed         | transferring                 | unavailable        |
      | failed         | awaiting_durable_receipts    | ambiguous          |
      | failed         | committing_cursor            | unavailable        |
      | cancelled      | requested                    | ambiguous          |
      | cancelled      | evaluating                   | unavailable        |

  @continuity @persistence @idempotence
  Scenario Outline: Retry the exact pending continuity commit idempotently
    Given one immutable pending "<candidate-kind>" commit for session "session-002"
    When "retry_pending_continuity" submits the exact candidate and continuity reports "<disposition>"
    Then no new SyncSession identity is issued
    And the candidate session ID, prior state, cursor expectation, receipts, and summary remain byte-for-byte equivalent
    And the candidate terminal state is exposed for "session-002"

    Examples:
      | candidate-kind             | disposition              |
      | terminal summary           | stored                   |
      | terminal summary           | already_stored_same      |
      | atomic completed cursor    | committed                |
      | atomic completed cursor    | already_committed_same   |

  @continuity @persistence @conflict
  Scenario: Fail closed when pending continuity content changes
    Given one immutable pending continuity commit for session "session-002"
    When "retry_pending_continuity" submits conflicting cursor, receipt, or summary content for "session-002"
    Then continuity refuses the candidate as "conflicting_continuity_content"
    And the session remains in its prior lifecycle state
    And no new SyncSession identity is issued

  @retry @immutability
  Scenario Outline: Retry with a new immutable session
    Given prior session "session-001" is terminal in state "<prior-state>"
    When trigger "retry" requests another attempt referencing "session-001"
    Then a different SyncSession identity "session-002" is issued
    And "session-002" records previous session ID "session-001"
    And every field and terminal summary of "session-001" remains unchanged
    And "session-002" re-evaluates current authority, agreement, selection, transport evidence, and cursor

    Examples:
      | prior-state |
      | failed      |
      | blocked     |

  @cancellation @safe-boundary
  Scenario Outline: Cancel only before checkpoint-unsafe work starts
    Given a session is in "<safe-state>"
    When Principal Node quiescence requests cancellation
    Then one immutable cancelled terminal summary is durably committed
    And transition "cancel_at_safe_boundary" exposes the session moving from "<safe-state>" to "cancelled"
    And no batch or cursor is partially committed

    Examples:
      | safe-state |
      | requested  |
      | evaluating |

  @cancellation @invalid-transition
  Scenario Outline: Refuse cancellation at a checkpoint-unsafe boundary
    Given a session is in "<unsafe-state>"
    When Principal Node quiescence requests cancellation
    Then transition "cancel_at_safe_boundary" is refused as "unsafe_cancellation_boundary"
    And the session does not enter "cancelled"
    And its batch, receipts, and cursor follow the ordinary fail-closed lifecycle

    Examples:
      | unsafe-state                |
      | transferring                |
      | awaiting_durable_receipts   |
      | committing_cursor           |

  @lifecycle @valid-transition
  Scenario Outline: Cover every declared valid lifecycle transition
    Given a SyncSession is in "<from>"
    And transition guards including "<durability>" are satisfied
    When transition "<transition>" is committed
    Then the session exposes state "<to>"

    Examples:
      | transition                    | from                       | to                         | durability                         |
      | begin_evaluation              | requested                  | evaluating                 | no terminal persistence            |
      | no_work_needed                | evaluating                 | completed                  | completed summary durable           |
      | begin_transfer                | evaluating                 | transferring               | no terminal persistence            |
      | begin_cursor_commit           | evaluating                 | committing_cursor          | no terminal persistence            |
      | refuse_transfer               | evaluating                 | blocked                    | blocked summary durable             |
      | evaluation_failure            | evaluating                 | failed                     | granular failed summary durable     |
      | batch_delivered               | transferring               | awaiting_durable_receipts  | no terminal persistence            |
      | transport_failure             | transferring               | failed                     | failed summary durable              |
      | receipts_checkpoint_safe      | awaiting_durable_receipts  | committing_cursor          | no terminal persistence            |
      | receipts_not_checkpoint_safe  | awaiting_durable_receipts  | failed                     | failed summary durable              |
      | cursor_durable                | committing_cursor           | completed                  | cursor and summary atomic durable   |
      | cursor_write_failed           | committing_cursor           | failed                     | failed summary durable              |
      | cancel_at_safe_boundary       | requested or evaluating     | cancelled                  | cancelled summary durable           |

  @lifecycle @invalid-transition
  Scenario Outline: Reject every declared invalid lifecycle transition
    Given a SyncSession at the relevant source state
    When "<invalid-transition>" is attempted
    Then it is refused as "<reason>"
    And no forbidden authority or durable state change occurs

    Examples:
      | invalid-transition                    | reason                              |
      | wake_directly_to_transfer             | evaluation_required                 |
      | host_commit_cursor                    | principal_node_owns_cursor_commit   |
      | rejected_receipt_to_cursor_commit     | receipt_not_checkpoint_safe         |
      | cross_source_cursor_reuse             | cursor_source_mismatch              |
      | transfer_rewritten_event              | exact_envelope_required              |
      | retry_from_host_scheduler_history     | principal_node_evidence_required    |
      | mutate_terminal_attempt_for_retry     | terminal_session_immutable          |
      | expose_terminal_without_durable_summary | durable_terminal_summary_required |
      | commit_cursor_without_completed_summary | atomic_continuity_commit_required  |
      | cancel_from_transferring              | unsafe_cancellation_boundary        |
      | cancel_from_awaiting_durable_receipts | unsafe_cancellation_boundary        |
      | cancel_from_committing_cursor         | unsafe_cancellation_boundary        |

  @architecture @static-conformance
  Scenario: Keep application and domain code technology-neutral and reviewable
    When the Principal Node synchronization implementation is inspected
    Then its application and domain code contains no launchd behavior
    And it contains no filesystem-path behavior
    And it contains no HTTP-client or WebSocket behavior
    And it contains no process-spawn behavior
    And its public API has documentation
    And its production paths introduce no "unwrap()" or "expect()"
    And scheduling, retry cadence, transport sockets, and OS supervision remain outside the portable relay core
