<!--
  generated from conductor v1
  model digest ad48b54e5a5c7a5d26726034920a9e4f29b75cf4fcd6e4d110ea66a2c8f4c0ee
  contract digest 9a2f202e3be77909ed7d34751b7e034f9ab12fe69955c93088bd68334629114c
  do not edit: regenerate with `ess synthesize --layout crate`
-->
# Synthesis plan — conductor v1

Scope: `component-skeletons`, laid out as `crate`, planned by `ess-synth`. Regenerate with `ess synthesize --layout crate`.

354 capabilities: **345 generated**, **1 obligations**, **8 refused**. An obligation is yours to implement against its contract; a refusal is a fact about this synthesis scope, not about the specification.

## Generated

| capability | source |
| --- | --- |
| domain type | `conductor.config.Authority` |
| domain type | `conductor.config.Cadence` |
| domain type | `conductor.config.Catalog` |
| domain type | `conductor.config.CatalogNames` |
| domain type | `conductor.config.Checkouts` |
| domain type | `conductor.config.CommandWord` |
| domain type | `conductor.config.ConductorSession` |
| domain type | `conductor.config.Config` |
| domain type | `conductor.config.Controllers` |
| domain type | `conductor.config.Gibibytes` |
| domain type | `conductor.config.GitHubSource` |
| domain type | `conductor.config.GitLabSource` |
| domain type | `conductor.config.Harness` |
| domain type | `conductor.config.InitRecords` |
| domain type | `conductor.config.Instance` |
| domain type | `conductor.config.InstanceName` |
| domain type | `conductor.config.LocalSource` |
| domain type | `conductor.config.Prerequisite` |
| domain type | `conductor.config.PrerequisiteState` |
| domain type | `conductor.config.Report` |
| domain type | `conductor.config.RepositoryRule` |
| domain type | `conductor.config.Retention` |
| domain type | `conductor.config.Role` |
| domain type | `conductor.config.SessionName` |
| domain type | `conductor.config.SessionPrefix` |
| domain type | `conductor.config.ShowConfig` |
| domain type | `conductor.config.Source` |
| domain type | `conductor.config.Thresholds` |
| domain type | `conductor.config.TimeOfDay` |
| domain type | `conductor.config.Tokens` |
| domain type | `conductor.config.TrustWorkspaces` |
| domain type | `conductor.config.ValidateConfig` |
| domain type | `conductor.decision.Decider` |
| domain type | `conductor.decision.Decision.State` |
| domain type | `conductor.decision.DecisionClass` |
| domain type | `conductor.decision.DecisionId` |
| domain type | `conductor.decision.DecisionRequest.State` |
| domain type | `conductor.decision.RequestId` |
| domain type | `conductor.decision.ShowDecision` |
| domain type | `conductor.direction.Activity` |
| domain type | `conductor.direction.DecidedBy` |
| domain type | `conductor.direction.Goal.State` |
| domain type | `conductor.direction.GoalId` |
| domain type | `conductor.direction.GoalServing.State` |
| domain type | `conductor.direction.MarkedBy` |
| domain type | `conductor.direction.MarkedRepository` |
| domain type | `conductor.direction.RepositoryActivity` |
| domain type | `conductor.direction.RepositoryActivityRow` |
| domain type | `conductor.direction.RepositoryMark.State` |
| domain type | `conductor.direction.ServingId` |
| domain type | `conductor.dispatch.Controller.State` |
| domain type | `conductor.dispatch.ControllerId` |
| domain type | `conductor.dispatch.Dispatch.State` |
| domain type | `conductor.dispatch.DispatchId` |
| domain type | `conductor.dispatch.GuardDecision.State` |
| domain type | `conductor.dispatch.GuardDecisionId` |
| domain type | `conductor.dispatch.Message.State` |
| domain type | `conductor.dispatch.MessageId` |
| domain type | `conductor.dispatch.MessageKind` |
| domain type | `conductor.dispatch.ResourceKind` |
| domain type | `conductor.dispatch.ResourceRequest.State` |
| domain type | `conductor.dispatch.ResourceRequestId` |
| domain type | `conductor.dispatch.Transport` |
| domain type | `conductor.dispatch.Verdict` |
| domain type | `conductor.observation.BlockerObservation.State` |
| domain type | `conductor.observation.Board.State` |
| domain type | `conductor.observation.BoardId` |
| domain type | `conductor.observation.CommitSha` |
| domain type | `conductor.observation.DashboardServe` |
| domain type | `conductor.observation.Harness` |
| domain type | `conductor.observation.Mergeability` |
| domain type | `conductor.observation.MergedPullRequestObservation.State` |
| domain type | `conductor.observation.ObservationId` |
| domain type | `conductor.observation.PullRequestObservation.State` |
| domain type | `conductor.observation.ReleaseObservation.State` |
| domain type | `conductor.observation.RepositoryName` |
| domain type | `conductor.observation.RepositoryObservation.State` |
| domain type | `conductor.observation.RepositoryShipped` |
| domain type | `conductor.observation.ResourceUsage` |
| domain type | `conductor.observation.RunConclusion` |
| domain type | `conductor.observation.SessionObservation.State` |
| domain type | `conductor.observation.SessionState` |
| domain type | `conductor.observation.Snapshot.State` |
| domain type | `conductor.observation.SnapshotId` |
| domain type | `conductor.observation.SpecificationObservation.State` |
| domain type | `conductor.observation.SpecificationPresence` |
| domain type | `conductor.observation.StoreMigrate` |
| domain type | `conductor.observation.ValidationResult` |
| domain type | `conductor.observation.Visibility` |
| domain type | `conductor.observation.WatchRun` |
| domain type | `conductor.observation.WorkflowRunObservation.State` |
| entity lifecycle | `conductor.decision.Decision` |
| entity lifecycle | `conductor.decision.DecisionRequest` |
| entity lifecycle | `conductor.direction.Goal` |
| entity lifecycle | `conductor.direction.GoalServing` |
| entity lifecycle | `conductor.direction.RepositoryMark` |
| entity lifecycle | `conductor.dispatch.Controller` |
| entity lifecycle | `conductor.dispatch.Dispatch` |
| entity lifecycle | `conductor.dispatch.GuardDecision` |
| entity lifecycle | `conductor.dispatch.Message` |
| entity lifecycle | `conductor.dispatch.ResourceRequest` |
| entity lifecycle | `conductor.observation.BlockerObservation` |
| entity lifecycle | `conductor.observation.Board` |
| entity lifecycle | `conductor.observation.MergedPullRequestObservation` |
| entity lifecycle | `conductor.observation.PullRequestObservation` |
| entity lifecycle | `conductor.observation.ReleaseObservation` |
| entity lifecycle | `conductor.observation.RepositoryObservation` |
| entity lifecycle | `conductor.observation.SessionObservation` |
| entity lifecycle | `conductor.observation.Snapshot` |
| entity lifecycle | `conductor.observation.SpecificationObservation` |
| entity lifecycle | `conductor.observation.WorkflowRunObservation` |
| command contract | `conductor.decision.AnswerEscalatedRequest` |
| command behaviour | `conductor.decision.AnswerEscalatedRequest` |
| command contract | `conductor.decision.AnswerRequest` |
| command behaviour | `conductor.decision.AnswerRequest` |
| command contract | `conductor.decision.EscalateRequest` |
| command behaviour | `conductor.decision.EscalateRequest` |
| command contract | `conductor.decision.RaiseDecisionRequest` |
| command behaviour | `conductor.decision.RaiseDecisionRequest` |
| command contract | `conductor.decision.RecordConductorDecision` |
| command behaviour | `conductor.decision.RecordConductorDecision` |
| command contract | `conductor.decision.RecordOperatorDecision` |
| command behaviour | `conductor.decision.RecordOperatorDecision` |
| command contract | `conductor.decision.ReverseConductorDecision` |
| command behaviour | `conductor.decision.ReverseConductorDecision` |
| command contract | `conductor.decision.ReverseOperatorDecision` |
| command behaviour | `conductor.decision.ReverseOperatorDecision` |
| command contract | `conductor.decision.WithdrawRequest` |
| command behaviour | `conductor.decision.WithdrawRequest` |
| command contract | `conductor.direction.ActivateRepository` |
| command behaviour | `conductor.direction.ActivateRepository` |
| command contract | `conductor.direction.AddServing` |
| command behaviour | `conductor.direction.AddServing` |
| command contract | `conductor.direction.ConfirmGoal` |
| command behaviour | `conductor.direction.ConfirmGoal` |
| command contract | `conductor.direction.DeactivateRepository` |
| command behaviour | `conductor.direction.DeactivateRepository` |
| command contract | `conductor.direction.DropGoal` |
| command behaviour | `conductor.direction.DropGoal` |
| command contract | `conductor.direction.MarkGoalMet` |
| command behaviour | `conductor.direction.MarkGoalMet` |
| command contract | `conductor.direction.MarkRepository` |
| command behaviour | `conductor.direction.MarkRepository` |
| command contract | `conductor.direction.ProposeGoal` |
| command behaviour | `conductor.direction.ProposeGoal` |
| command contract | `conductor.direction.RemoveServing` |
| command behaviour | `conductor.direction.RemoveServing` |
| command contract | `conductor.dispatch.CancelDispatch` |
| command behaviour | `conductor.dispatch.CancelDispatch` |
| command contract | `conductor.dispatch.GrantResource` |
| command behaviour | `conductor.dispatch.GrantResource` |
| command contract | `conductor.dispatch.HandleMessage` |
| command behaviour | `conductor.dispatch.HandleMessage` |
| command contract | `conductor.dispatch.PauseController` |
| command behaviour | `conductor.dispatch.PauseController` |
| command contract | `conductor.dispatch.ReceiveMessage` |
| command behaviour | `conductor.dispatch.ReceiveMessage` |
| command contract | `conductor.dispatch.RecordGuardDecision` |
| command behaviour | `conductor.dispatch.RecordGuardDecision` |
| command contract | `conductor.dispatch.RefuseResource` |
| command behaviour | `conductor.dispatch.RefuseResource` |
| command contract | `conductor.dispatch.RejectMessage` |
| command behaviour | `conductor.dispatch.RejectMessage` |
| command contract | `conductor.dispatch.ReleaseResource` |
| command behaviour | `conductor.dispatch.ReleaseResource` |
| command contract | `conductor.dispatch.ReportBlocked` |
| command behaviour | `conductor.dispatch.ReportBlocked` |
| command contract | `conductor.dispatch.ReportDone` |
| command behaviour | `conductor.dispatch.ReportDone` |
| command contract | `conductor.dispatch.ReportFailed` |
| command behaviour | `conductor.dispatch.ReportFailed` |
| command contract | `conductor.dispatch.ReportStarted` |
| command behaviour | `conductor.dispatch.ReportStarted` |
| command contract | `conductor.dispatch.ReportUnblocked` |
| command behaviour | `conductor.dispatch.ReportUnblocked` |
| command contract | `conductor.dispatch.RequestResource` |
| command behaviour | `conductor.dispatch.RequestResource` |
| command contract | `conductor.dispatch.ResumeController` |
| command behaviour | `conductor.dispatch.ResumeController` |
| command contract | `conductor.dispatch.ReviseCharter` |
| command behaviour | `conductor.dispatch.ReviseCharter` |
| command contract | `conductor.dispatch.RouteNeed` |
| command behaviour | `conductor.dispatch.RouteNeed` |
| command contract | `conductor.dispatch.SendDispatch` |
| command behaviour | `conductor.dispatch.SendDispatch` |
| command contract | `conductor.dispatch.StartController` |
| command contract | `conductor.dispatch.StopController` |
| command behaviour | `conductor.dispatch.StopController` |
| command contract | `conductor.observation.CompleteSnapshot` |
| command behaviour | `conductor.observation.CompleteSnapshot` |
| command contract | `conductor.observation.FailSnapshot` |
| command behaviour | `conductor.observation.FailSnapshot` |
| command contract | `conductor.observation.PublishBoard` |
| command behaviour | `conductor.observation.PublishBoard` |
| command contract | `conductor.observation.RecordBlocker` |
| command behaviour | `conductor.observation.RecordBlocker` |
| command contract | `conductor.observation.RecordMergedPullRequest` |
| command behaviour | `conductor.observation.RecordMergedPullRequest` |
| command contract | `conductor.observation.RecordPullRequest` |
| command behaviour | `conductor.observation.RecordPullRequest` |
| command contract | `conductor.observation.RecordRelease` |
| command behaviour | `conductor.observation.RecordRelease` |
| command contract | `conductor.observation.RecordRepository` |
| command behaviour | `conductor.observation.RecordRepository` |
| command contract | `conductor.observation.RecordSession` |
| command behaviour | `conductor.observation.RecordSession` |
| command contract | `conductor.observation.RecordSpecification` |
| command behaviour | `conductor.observation.RecordSpecification` |
| command contract | `conductor.observation.RecordWorkflowRun` |
| command behaviour | `conductor.observation.RecordWorkflowRun` |
| command contract | `conductor.observation.StartSnapshot` |
| command behaviour | `conductor.observation.StartSnapshot` |
| event type | `conductor.decision.DecisionRecorded` |
| event type | `conductor.decision.DecisionRequested` |
| event type | `conductor.decision.DecisionReversedEvent` |
| event type | `conductor.decision.RequestAnswered` |
| event type | `conductor.decision.RequestEscalated` |
| event type | `conductor.decision.RequestWithdrawn` |
| event type | `conductor.direction.GoalConfirmed` |
| event type | `conductor.direction.GoalDropped` |
| event type | `conductor.direction.GoalMet` |
| event type | `conductor.direction.GoalProposed` |
| event type | `conductor.direction.RepositoryActivated` |
| event type | `conductor.direction.RepositoryDeactivated` |
| event type | `conductor.direction.RepositoryMarked` |
| event type | `conductor.direction.ServingAdded` |
| event type | `conductor.direction.ServingRemoved` |
| event type | `conductor.dispatch.CharterRevised` |
| event type | `conductor.dispatch.ControllerPaused` |
| event type | `conductor.dispatch.ControllerResumed` |
| event type | `conductor.dispatch.ControllerStarted` |
| event type | `conductor.dispatch.ControllerStopped` |
| event type | `conductor.dispatch.DispatchBlocked` |
| event type | `conductor.dispatch.DispatchCancelled` |
| event type | `conductor.dispatch.DispatchDone` |
| event type | `conductor.dispatch.DispatchFailed` |
| event type | `conductor.dispatch.DispatchSent` |
| event type | `conductor.dispatch.DispatchStarted` |
| event type | `conductor.dispatch.DispatchUnblocked` |
| event type | `conductor.dispatch.GuardDecisionRecorded` |
| event type | `conductor.dispatch.MessageHandled` |
| event type | `conductor.dispatch.MessageReceived` |
| event type | `conductor.dispatch.MessageRejected` |
| event type | `conductor.dispatch.NeedRouted` |
| event type | `conductor.dispatch.ResourceGranted` |
| event type | `conductor.dispatch.ResourceRefused` |
| event type | `conductor.dispatch.ResourceReleased` |
| event type | `conductor.dispatch.ResourceRequested` |
| event type | `conductor.observation.BlockerRecorded` |
| event type | `conductor.observation.BoardPublished` |
| event type | `conductor.observation.MergedPullRequestRecorded` |
| event type | `conductor.observation.PullRequestRecorded` |
| event type | `conductor.observation.ReleaseRecorded` |
| event type | `conductor.observation.RepositoryRecorded` |
| event type | `conductor.observation.SessionRecorded` |
| event type | `conductor.observation.SnapshotCompleted` |
| event type | `conductor.observation.SnapshotFailed` |
| event type | `conductor.observation.SnapshotStarted` |
| event type | `conductor.observation.SpecificationRecorded` |
| event type | `conductor.observation.WorkflowRunRecorded` |
| error type | `conductor.decision.DecisionExists` |
| error type | `conductor.decision.DecisionNotFound` |
| error type | `conductor.decision.DecisionReversed` |
| error type | `conductor.decision.DecisionStateConflict` |
| error type | `conductor.decision.NotEscalatable` |
| error type | `conductor.decision.RequestExists` |
| error type | `conductor.decision.RequestNotFound` |
| error type | `conductor.decision.RequestStateConflict` |
| error type | `conductor.decision.ReservedForOperator` |
| error type | `conductor.decision.ReversalReservedForOperator` |
| error type | `conductor.direction.GoalExists` |
| error type | `conductor.direction.GoalNotFound` |
| error type | `conductor.direction.GoalStateConflict` |
| error type | `conductor.direction.InvalidGoalId` |
| error type | `conductor.direction.RepositoryAlreadyMarked` |
| error type | `conductor.direction.RepositoryMarkConflict` |
| error type | `conductor.direction.RepositoryMarkNotFound` |
| error type | `conductor.direction.ServingNotFound` |
| error type | `conductor.direction.ServingStateConflict` |
| error type | `conductor.dispatch.ControllerAlreadyRunning` |
| error type | `conductor.dispatch.ControllerNotFound` |
| error type | `conductor.dispatch.ControllerStateConflict` |
| error type | `conductor.dispatch.DiskBelowFloor` |
| error type | `conductor.dispatch.DispatchExists` |
| error type | `conductor.dispatch.DispatchNotFound` |
| error type | `conductor.dispatch.DispatchStateConflict` |
| error type | `conductor.dispatch.GoalClosed` |
| error type | `conductor.dispatch.GoalNotFound` |
| error type | `conductor.dispatch.InvalidAmount` |
| error type | `conductor.dispatch.InvalidPriority` |
| error type | `conductor.dispatch.MessageNotFound` |
| error type | `conductor.dispatch.MessageStateConflict` |
| error type | `conductor.dispatch.NeedNotRouted` |
| error type | `conductor.dispatch.NotANeed` |
| error type | `conductor.dispatch.ResourceRequestNotFound` |
| error type | `conductor.dispatch.ResourceRequestStateConflict` |
| error type | `conductor.dispatch.WaitsOnItself` |
| error type | `conductor.observation.InvalidCount` |
| error type | `conductor.observation.InvalidPullRequestNumber` |
| error type | `conductor.observation.SnapshotNotCollecting` |
| error type | `conductor.observation.SnapshotNotComplete` |
| error type | `conductor.observation.SnapshotNotFound` |
| view type | `conductor.decision.Decisions` |
| view query | `conductor.decision.Decisions` |
| view type | `conductor.decision.HandsTodo` |
| view query | `conductor.decision.HandsTodo` |
| view type | `conductor.decision.Requests` |
| view query | `conductor.decision.Requests` |
| view type | `conductor.direction.GoalServings` |
| view query | `conductor.direction.GoalServings` |
| view type | `conductor.direction.Goals` |
| view query | `conductor.direction.Goals` |
| view type | `conductor.direction.RepositoryMarks` |
| view query | `conductor.direction.RepositoryMarks` |
| view type | `conductor.dispatch.Controllers` |
| view query | `conductor.dispatch.Controllers` |
| view type | `conductor.dispatch.Dispatches` |
| view query | `conductor.dispatch.Dispatches` |
| view type | `conductor.dispatch.GuardDecisions` |
| view query | `conductor.dispatch.GuardDecisions` |
| view type | `conductor.dispatch.Messages` |
| view query | `conductor.dispatch.Messages` |
| view type | `conductor.dispatch.ResourceRequests` |
| view query | `conductor.dispatch.ResourceRequests` |
| view type | `conductor.observation.Blockers` |
| view query | `conductor.observation.Blockers` |
| view type | `conductor.observation.Boards` |
| view query | `conductor.observation.Boards` |
| view type | `conductor.observation.MergedPullRequests` |
| view query | `conductor.observation.MergedPullRequests` |
| view type | `conductor.observation.PullRequests` |
| view query | `conductor.observation.PullRequests` |
| view type | `conductor.observation.Releases` |
| view query | `conductor.observation.Releases` |
| view type | `conductor.observation.Repositories` |
| view query | `conductor.observation.Repositories` |
| view type | `conductor.observation.Sessions` |
| view query | `conductor.observation.Sessions` |
| view type | `conductor.observation.Snapshots` |
| view query | `conductor.observation.Snapshots` |
| view type | `conductor.observation.Specifications` |
| view query | `conductor.observation.Specifications` |
| view type | `conductor.observation.WorkflowRuns` |
| view query | `conductor.observation.WorkflowRuns` |
| component port | `conductor-cli` |

## Ports — yours to provide

What the specification fully determines is generated; what it cannot determine is an obligation. A generated command behaviour or view query reads and writes through the ports below, and they are yours to provide: synthesis generates each port's contract and never an implementation of one, so where instances live stays your decision.

| port | what it answers |
| --- | --- |
| storage | one per entity a generated behaviour or query reads or writes: the instance stored under an identity; storing, replacing and removing one; and every stored instance, in the order the store keeps them |
| context | where a generated behaviour asks it: the caller's attributes, every identity and value the specification says the implementation assigns, and whether each `external:` branch is taken |

## Obligations — yours to implement

| capability | source | why not generated | contract |
| --- | --- | --- | --- |
| command behaviour | `conductor.dispatch.StartController` | kept an obligation by a guard over the rows a selector selects (`when_related: {entity, where, exists | count | forall}`), in `already-running` | given `conductor.dispatch.StartController` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `already-running` when, of the `conductor.dispatch.Controller` rows satisfying `(repository == input.repository and state in [Running, Paused])`, at least 1 rows are selected, error `conductor.dispatch.ControllerAlreadyRunning`; `started` otherwise, creates `conductor.dispatch.Controller`, emits `conductor.dispatch.ControllerStarted` |

## Refused — not represented by this synthesis

| capability | source | stage | why |
| --- | --- | --- | --- |
| actor grants | `conductor.decision.Conductor` | planning | may invoke `conductor.decision.AnswerRequest`, `conductor.decision.EscalateRequest`, `conductor.decision.RecordConductorDecision`, `conductor.decision.ReverseConductorDecision`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
| actor grants | `conductor.decision.Operator` | planning | may invoke `conductor.decision.AnswerEscalatedRequest`, `conductor.decision.RecordOperatorDecision`, `conductor.decision.ReverseOperatorDecision`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
| actor grants | `conductor.decision.RepositoryController` | planning | may invoke `conductor.decision.RaiseDecisionRequest`, `conductor.decision.WithdrawRequest`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
| actor grants | `conductor.direction.Conductor` | planning | may invoke `conductor.direction.ActivateRepository`, `conductor.direction.AddServing`, `conductor.direction.DeactivateRepository`, `conductor.direction.MarkGoalMet`, `conductor.direction.MarkRepository`, `conductor.direction.ProposeGoal`, `conductor.direction.RemoveServing`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
| actor grants | `conductor.direction.Operator` | planning | may invoke `conductor.direction.ActivateRepository`, `conductor.direction.ConfirmGoal`, `conductor.direction.DeactivateRepository`, `conductor.direction.DropGoal`, `conductor.direction.MarkRepository`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
| actor grants | `conductor.dispatch.Conductor` | planning | may invoke `conductor.dispatch.CancelDispatch`, `conductor.dispatch.GrantResource`, `conductor.dispatch.HandleMessage`, `conductor.dispatch.PauseController`, `conductor.dispatch.ReceiveMessage`, `conductor.dispatch.RefuseResource`, `conductor.dispatch.RejectMessage`, `conductor.dispatch.ReportBlocked`, `conductor.dispatch.ReportDone`, `conductor.dispatch.ReportFailed`, `conductor.dispatch.ReportStarted`, `conductor.dispatch.ReportUnblocked`, `conductor.dispatch.ResumeController`, `conductor.dispatch.ReviseCharter`, `conductor.dispatch.RouteNeed`, `conductor.dispatch.SendDispatch`, `conductor.dispatch.StartController`, `conductor.dispatch.StopController`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
| actor grants | `conductor.dispatch.RepositoryController` | planning | may invoke `conductor.dispatch.RecordGuardDecision`, `conductor.dispatch.ReleaseResource`, `conductor.dispatch.ReportBlocked`, `conductor.dispatch.ReportDone`, `conductor.dispatch.ReportFailed`, `conductor.dispatch.ReportStarted`, `conductor.dispatch.ReportUnblocked`, `conductor.dispatch.RequestResource`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
| actor grants | `conductor.observation.Collector` | planning | may invoke `conductor.observation.CompleteSnapshot`, `conductor.observation.FailSnapshot`, `conductor.observation.PublishBoard`, `conductor.observation.RecordBlocker`, `conductor.observation.RecordMergedPullRequest`, `conductor.observation.RecordPullRequest`, `conductor.observation.RecordRelease`, `conductor.observation.RecordRepository`, `conductor.observation.RecordSession`, `conductor.observation.RecordSpecification`, `conductor.observation.RecordWorkflowRun`, `conductor.observation.StartSnapshot`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
