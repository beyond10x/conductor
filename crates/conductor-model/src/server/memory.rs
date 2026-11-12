// generated from conductor v1
// contract digest b817c831ddcdf9949972f6a946145799d67909f4f14e497f4a16667b7861cfad
// do not edit: regenerate with `ess synthesize --layout crate`

//! Ephemeral stores for the generated network entry points; no durability.

/// Ephemeral storage of `conductor.agent.Agent`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryAgentStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::agent::AgentSnapshot>>>);
impl crate::behaviour::AgentStorage for MemoryAgentStorage {
fn get(&self, identity: &crate::agent::AgentName) -> Option<crate::agent::AgentSnapshot> { let key = memory_key_3(identity); self.0.borrow().iter().find(|row| memory_key_3(&row.data.name) == key).cloned() }
fn put(&mut self, snapshot: crate::agent::AgentSnapshot) { self.delete(&snapshot.data.name); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_3(&row.data.name).cmp(&memory_key_3(&other.data.name))); }
fn delete(&mut self, identity: &crate::agent::AgentName) { let key = memory_key_3(identity); self.0.borrow_mut().retain(|row| memory_key_3(&row.data.name) != key); }
fn list(&self) -> Vec<crate::agent::AgentSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::AgentStorage for MemoryPorts {
fn get(&self, identity: &crate::agent::AgentName) -> Option<crate::agent::AgentSnapshot> { crate::behaviour::AgentStorage::get(&self.agent_storage, identity) }
fn put(&mut self, snapshot: crate::agent::AgentSnapshot) { crate::behaviour::AgentStorage::put(&mut self.agent_storage, snapshot); }
fn delete(&mut self, identity: &crate::agent::AgentName) { crate::behaviour::AgentStorage::delete(&mut self.agent_storage, identity); }
fn list(&self) -> Vec<crate::agent::AgentSnapshot> { crate::behaviour::AgentStorage::list(&self.agent_storage) }
}

/// Ephemeral storage of `conductor.agent.AgentKind`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryAgentKindStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::agent::AgentKindSnapshot>>>);
impl crate::behaviour::AgentKindStorage for MemoryAgentKindStorage {
fn get(&self, identity: &crate::agent::AgentKindName) -> Option<crate::agent::AgentKindSnapshot> { let key = memory_key_2(identity); self.0.borrow().iter().find(|row| memory_key_2(&row.data.name) == key).cloned() }
fn put(&mut self, snapshot: crate::agent::AgentKindSnapshot) { self.delete(&snapshot.data.name); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_2(&row.data.name).cmp(&memory_key_2(&other.data.name))); }
fn delete(&mut self, identity: &crate::agent::AgentKindName) { let key = memory_key_2(identity); self.0.borrow_mut().retain(|row| memory_key_2(&row.data.name) != key); }
fn list(&self) -> Vec<crate::agent::AgentKindSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::AgentKindStorage for MemoryPorts {
fn get(&self, identity: &crate::agent::AgentKindName) -> Option<crate::agent::AgentKindSnapshot> { crate::behaviour::AgentKindStorage::get(&self.agent_kind_storage, identity) }
fn put(&mut self, snapshot: crate::agent::AgentKindSnapshot) { crate::behaviour::AgentKindStorage::put(&mut self.agent_kind_storage, snapshot); }
fn delete(&mut self, identity: &crate::agent::AgentKindName) { crate::behaviour::AgentKindStorage::delete(&mut self.agent_kind_storage, identity); }
fn list(&self) -> Vec<crate::agent::AgentKindSnapshot> { crate::behaviour::AgentKindStorage::list(&self.agent_kind_storage) }
}

/// Ephemeral storage of `conductor.config.ConfigRevision`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryConfigRevisionStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::config::ConfigRevisionSnapshot>>>);
impl crate::behaviour::ConfigRevisionStorage for MemoryConfigRevisionStorage {
fn get(&self, identity: &crate::config::RevisionId) -> Option<crate::config::ConfigRevisionSnapshot> { let key = memory_key_74(identity); self.0.borrow().iter().find(|row| memory_key_74(&row.data.revision_id) == key).cloned() }
fn put(&mut self, snapshot: crate::config::ConfigRevisionSnapshot) { self.delete(&snapshot.data.revision_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_74(&row.data.revision_id).cmp(&memory_key_74(&other.data.revision_id))); }
fn delete(&mut self, identity: &crate::config::RevisionId) { let key = memory_key_74(identity); self.0.borrow_mut().retain(|row| memory_key_74(&row.data.revision_id) != key); }
fn list(&self) -> Vec<crate::config::ConfigRevisionSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::ConfigRevisionStorage for MemoryPorts {
fn get(&self, identity: &crate::config::RevisionId) -> Option<crate::config::ConfigRevisionSnapshot> { crate::behaviour::ConfigRevisionStorage::get(&self.config_revision_storage, identity) }
fn put(&mut self, snapshot: crate::config::ConfigRevisionSnapshot) { crate::behaviour::ConfigRevisionStorage::put(&mut self.config_revision_storage, snapshot); }
fn delete(&mut self, identity: &crate::config::RevisionId) { crate::behaviour::ConfigRevisionStorage::delete(&mut self.config_revision_storage, identity); }
fn list(&self) -> Vec<crate::config::ConfigRevisionSnapshot> { crate::behaviour::ConfigRevisionStorage::list(&self.config_revision_storage) }
}

/// Ephemeral storage of `conductor.decision.Decision`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryDecisionStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::decision::DecisionSnapshot>>>);
impl crate::behaviour::DecisionStorage for MemoryDecisionStorage {
fn get(&self, identity: &crate::decision::DecisionId) -> Option<crate::decision::DecisionSnapshot> { let key = memory_key_108(identity); self.0.borrow().iter().find(|row| memory_key_108(&row.data.decision_id) == key).cloned() }
fn put(&mut self, snapshot: crate::decision::DecisionSnapshot) { self.delete(&snapshot.data.decision_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_108(&row.data.decision_id).cmp(&memory_key_108(&other.data.decision_id))); }
fn delete(&mut self, identity: &crate::decision::DecisionId) { let key = memory_key_108(identity); self.0.borrow_mut().retain(|row| memory_key_108(&row.data.decision_id) != key); }
fn list(&self) -> Vec<crate::decision::DecisionSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::DecisionStorage for MemoryPorts {
fn get(&self, identity: &crate::decision::DecisionId) -> Option<crate::decision::DecisionSnapshot> { crate::behaviour::DecisionStorage::get(&self.decision_storage, identity) }
fn put(&mut self, snapshot: crate::decision::DecisionSnapshot) { crate::behaviour::DecisionStorage::put(&mut self.decision_storage, snapshot); }
fn delete(&mut self, identity: &crate::decision::DecisionId) { crate::behaviour::DecisionStorage::delete(&mut self.decision_storage, identity); }
fn list(&self) -> Vec<crate::decision::DecisionSnapshot> { crate::behaviour::DecisionStorage::list(&self.decision_storage) }
}

/// Ephemeral storage of `conductor.decision.DecisionRequest`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryDecisionRequestStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::decision::DecisionRequestSnapshot>>>);
impl crate::behaviour::DecisionRequestStorage for MemoryDecisionRequestStorage {
fn get(&self, identity: &crate::decision::RequestId) -> Option<crate::decision::DecisionRequestSnapshot> { let key = memory_key_114(identity); self.0.borrow().iter().find(|row| memory_key_114(&row.data.request_id) == key).cloned() }
fn put(&mut self, snapshot: crate::decision::DecisionRequestSnapshot) { self.delete(&snapshot.data.request_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_114(&row.data.request_id).cmp(&memory_key_114(&other.data.request_id))); }
fn delete(&mut self, identity: &crate::decision::RequestId) { let key = memory_key_114(identity); self.0.borrow_mut().retain(|row| memory_key_114(&row.data.request_id) != key); }
fn list(&self) -> Vec<crate::decision::DecisionRequestSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::DecisionRequestStorage for MemoryPorts {
fn get(&self, identity: &crate::decision::RequestId) -> Option<crate::decision::DecisionRequestSnapshot> { crate::behaviour::DecisionRequestStorage::get(&self.decision_request_storage, identity) }
fn put(&mut self, snapshot: crate::decision::DecisionRequestSnapshot) { crate::behaviour::DecisionRequestStorage::put(&mut self.decision_request_storage, snapshot); }
fn delete(&mut self, identity: &crate::decision::RequestId) { crate::behaviour::DecisionRequestStorage::delete(&mut self.decision_request_storage, identity); }
fn list(&self) -> Vec<crate::decision::DecisionRequestSnapshot> { crate::behaviour::DecisionRequestStorage::list(&self.decision_request_storage) }
}

/// Ephemeral storage of `conductor.direction.Goal`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryGoalStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::direction::GoalSnapshot>>>);
impl crate::behaviour::GoalStorage for MemoryGoalStorage {
fn get(&self, identity: &crate::direction::GoalId) -> Option<crate::direction::GoalSnapshot> { let key = memory_key_121(identity); self.0.borrow().iter().find(|row| memory_key_121(&row.data.goal_id) == key).cloned() }
fn put(&mut self, snapshot: crate::direction::GoalSnapshot) { self.delete(&snapshot.data.goal_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_121(&row.data.goal_id).cmp(&memory_key_121(&other.data.goal_id))); }
fn delete(&mut self, identity: &crate::direction::GoalId) { let key = memory_key_121(identity); self.0.borrow_mut().retain(|row| memory_key_121(&row.data.goal_id) != key); }
fn list(&self) -> Vec<crate::direction::GoalSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::GoalStorage for MemoryPorts {
fn get(&self, identity: &crate::direction::GoalId) -> Option<crate::direction::GoalSnapshot> { crate::behaviour::GoalStorage::get(&self.goal_storage, identity) }
fn put(&mut self, snapshot: crate::direction::GoalSnapshot) { crate::behaviour::GoalStorage::put(&mut self.goal_storage, snapshot); }
fn delete(&mut self, identity: &crate::direction::GoalId) { crate::behaviour::GoalStorage::delete(&mut self.goal_storage, identity); }
fn list(&self) -> Vec<crate::direction::GoalSnapshot> { crate::behaviour::GoalStorage::list(&self.goal_storage) }
}

/// Ephemeral storage of `conductor.direction.GoalServing`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryGoalServingStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::direction::GoalServingSnapshot>>>);
impl crate::behaviour::GoalServingStorage for MemoryGoalServingStorage {
fn get(&self, identity: &crate::direction::ServingId) -> Option<crate::direction::GoalServingSnapshot> { let key = memory_key_143(identity); self.0.borrow().iter().find(|row| memory_key_143(&row.data.serving_id) == key).cloned() }
fn put(&mut self, snapshot: crate::direction::GoalServingSnapshot) { self.delete(&snapshot.data.serving_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_143(&row.data.serving_id).cmp(&memory_key_143(&other.data.serving_id))); }
fn delete(&mut self, identity: &crate::direction::ServingId) { let key = memory_key_143(identity); self.0.borrow_mut().retain(|row| memory_key_143(&row.data.serving_id) != key); }
fn list(&self) -> Vec<crate::direction::GoalServingSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::GoalServingStorage for MemoryPorts {
fn get(&self, identity: &crate::direction::ServingId) -> Option<crate::direction::GoalServingSnapshot> { crate::behaviour::GoalServingStorage::get(&self.goal_serving_storage, identity) }
fn put(&mut self, snapshot: crate::direction::GoalServingSnapshot) { crate::behaviour::GoalServingStorage::put(&mut self.goal_serving_storage, snapshot); }
fn delete(&mut self, identity: &crate::direction::ServingId) { crate::behaviour::GoalServingStorage::delete(&mut self.goal_serving_storage, identity); }
fn list(&self) -> Vec<crate::direction::GoalServingSnapshot> { crate::behaviour::GoalServingStorage::list(&self.goal_serving_storage) }
}

/// Ephemeral storage of `conductor.direction.RepositoryMark`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryRepositoryMarkStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::direction::RepositoryMarkSnapshot>>>);
impl crate::behaviour::RepositoryMarkStorage for MemoryRepositoryMarkStorage {
fn get(&self, identity: &crate::direction::MarkedRepository) -> Option<crate::direction::RepositoryMarkSnapshot> { let key = memory_key_124(identity); self.0.borrow().iter().find(|row| memory_key_124(&row.data.repository) == key).cloned() }
fn put(&mut self, snapshot: crate::direction::RepositoryMarkSnapshot) { self.delete(&snapshot.data.repository); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_124(&row.data.repository).cmp(&memory_key_124(&other.data.repository))); }
fn delete(&mut self, identity: &crate::direction::MarkedRepository) { let key = memory_key_124(identity); self.0.borrow_mut().retain(|row| memory_key_124(&row.data.repository) != key); }
fn list(&self) -> Vec<crate::direction::RepositoryMarkSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::RepositoryMarkStorage for MemoryPorts {
fn get(&self, identity: &crate::direction::MarkedRepository) -> Option<crate::direction::RepositoryMarkSnapshot> { crate::behaviour::RepositoryMarkStorage::get(&self.repository_mark_storage, identity) }
fn put(&mut self, snapshot: crate::direction::RepositoryMarkSnapshot) { crate::behaviour::RepositoryMarkStorage::put(&mut self.repository_mark_storage, snapshot); }
fn delete(&mut self, identity: &crate::direction::MarkedRepository) { crate::behaviour::RepositoryMarkStorage::delete(&mut self.repository_mark_storage, identity); }
fn list(&self) -> Vec<crate::direction::RepositoryMarkSnapshot> { crate::behaviour::RepositoryMarkStorage::list(&self.repository_mark_storage) }
}

/// Ephemeral storage of `conductor.dispatch.Controller`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryControllerStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::dispatch::ControllerSnapshot>>>);
impl crate::behaviour::ControllerStorage for MemoryControllerStorage {
fn get(&self, identity: &crate::dispatch::ControllerId) -> Option<crate::dispatch::ControllerSnapshot> { let key = memory_key_147(identity); self.0.borrow().iter().find(|row| memory_key_147(&row.data.controller_id) == key).cloned() }
fn put(&mut self, snapshot: crate::dispatch::ControllerSnapshot) { self.delete(&snapshot.data.controller_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_147(&row.data.controller_id).cmp(&memory_key_147(&other.data.controller_id))); }
fn delete(&mut self, identity: &crate::dispatch::ControllerId) { let key = memory_key_147(identity); self.0.borrow_mut().retain(|row| memory_key_147(&row.data.controller_id) != key); }
fn list(&self) -> Vec<crate::dispatch::ControllerSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::ControllerStorage for MemoryPorts {
fn get(&self, identity: &crate::dispatch::ControllerId) -> Option<crate::dispatch::ControllerSnapshot> { crate::behaviour::ControllerStorage::get(&self.controller_storage, identity) }
fn put(&mut self, snapshot: crate::dispatch::ControllerSnapshot) { crate::behaviour::ControllerStorage::put(&mut self.controller_storage, snapshot); }
fn delete(&mut self, identity: &crate::dispatch::ControllerId) { crate::behaviour::ControllerStorage::delete(&mut self.controller_storage, identity); }
fn list(&self) -> Vec<crate::dispatch::ControllerSnapshot> { crate::behaviour::ControllerStorage::list(&self.controller_storage) }
}

/// Ephemeral storage of `conductor.dispatch.Dispatch`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryDispatchStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::dispatch::DispatchSnapshot>>>);
impl crate::behaviour::DispatchStorage for MemoryDispatchStorage {
fn get(&self, identity: &crate::dispatch::DispatchId) -> Option<crate::dispatch::DispatchSnapshot> { let key = memory_key_149(identity); self.0.borrow().iter().find(|row| memory_key_149(&row.data.dispatch_id) == key).cloned() }
fn put(&mut self, snapshot: crate::dispatch::DispatchSnapshot) { self.delete(&snapshot.data.dispatch_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_149(&row.data.dispatch_id).cmp(&memory_key_149(&other.data.dispatch_id))); }
fn delete(&mut self, identity: &crate::dispatch::DispatchId) { let key = memory_key_149(identity); self.0.borrow_mut().retain(|row| memory_key_149(&row.data.dispatch_id) != key); }
fn list(&self) -> Vec<crate::dispatch::DispatchSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::DispatchStorage for MemoryPorts {
fn get(&self, identity: &crate::dispatch::DispatchId) -> Option<crate::dispatch::DispatchSnapshot> { crate::behaviour::DispatchStorage::get(&self.dispatch_storage, identity) }
fn put(&mut self, snapshot: crate::dispatch::DispatchSnapshot) { crate::behaviour::DispatchStorage::put(&mut self.dispatch_storage, snapshot); }
fn delete(&mut self, identity: &crate::dispatch::DispatchId) { crate::behaviour::DispatchStorage::delete(&mut self.dispatch_storage, identity); }
fn list(&self) -> Vec<crate::dispatch::DispatchSnapshot> { crate::behaviour::DispatchStorage::list(&self.dispatch_storage) }
}

/// Ephemeral storage of `conductor.dispatch.GuardDecision`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryGuardDecisionStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::dispatch::GuardDecisionSnapshot>>>);
impl crate::behaviour::GuardDecisionStorage for MemoryGuardDecisionStorage {
fn get(&self, identity: &crate::dispatch::GuardDecisionId) -> Option<crate::dispatch::GuardDecisionSnapshot> { let key = memory_key_153(identity); self.0.borrow().iter().find(|row| memory_key_153(&row.data.guard_decision_id) == key).cloned() }
fn put(&mut self, snapshot: crate::dispatch::GuardDecisionSnapshot) { self.delete(&snapshot.data.guard_decision_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_153(&row.data.guard_decision_id).cmp(&memory_key_153(&other.data.guard_decision_id))); }
fn delete(&mut self, identity: &crate::dispatch::GuardDecisionId) { let key = memory_key_153(identity); self.0.borrow_mut().retain(|row| memory_key_153(&row.data.guard_decision_id) != key); }
fn list(&self) -> Vec<crate::dispatch::GuardDecisionSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::GuardDecisionStorage for MemoryPorts {
fn get(&self, identity: &crate::dispatch::GuardDecisionId) -> Option<crate::dispatch::GuardDecisionSnapshot> { crate::behaviour::GuardDecisionStorage::get(&self.guard_decision_storage, identity) }
fn put(&mut self, snapshot: crate::dispatch::GuardDecisionSnapshot) { crate::behaviour::GuardDecisionStorage::put(&mut self.guard_decision_storage, snapshot); }
fn delete(&mut self, identity: &crate::dispatch::GuardDecisionId) { crate::behaviour::GuardDecisionStorage::delete(&mut self.guard_decision_storage, identity); }
fn list(&self) -> Vec<crate::dispatch::GuardDecisionSnapshot> { crate::behaviour::GuardDecisionStorage::list(&self.guard_decision_storage) }
}

/// Ephemeral storage of `conductor.dispatch.Message`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryMessageStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::dispatch::MessageSnapshot>>>);
impl crate::behaviour::MessageStorage for MemoryMessageStorage {
fn get(&self, identity: &crate::dispatch::MessageId) -> Option<crate::dispatch::MessageSnapshot> { let key = memory_key_158(identity); self.0.borrow().iter().find(|row| memory_key_158(&row.data.message_id) == key).cloned() }
fn put(&mut self, snapshot: crate::dispatch::MessageSnapshot) { self.delete(&snapshot.data.message_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_158(&row.data.message_id).cmp(&memory_key_158(&other.data.message_id))); }
fn delete(&mut self, identity: &crate::dispatch::MessageId) { let key = memory_key_158(identity); self.0.borrow_mut().retain(|row| memory_key_158(&row.data.message_id) != key); }
fn list(&self) -> Vec<crate::dispatch::MessageSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::MessageStorage for MemoryPorts {
fn get(&self, identity: &crate::dispatch::MessageId) -> Option<crate::dispatch::MessageSnapshot> { crate::behaviour::MessageStorage::get(&self.message_storage, identity) }
fn put(&mut self, snapshot: crate::dispatch::MessageSnapshot) { crate::behaviour::MessageStorage::put(&mut self.message_storage, snapshot); }
fn delete(&mut self, identity: &crate::dispatch::MessageId) { crate::behaviour::MessageStorage::delete(&mut self.message_storage, identity); }
fn list(&self) -> Vec<crate::dispatch::MessageSnapshot> { crate::behaviour::MessageStorage::list(&self.message_storage) }
}

/// Ephemeral storage of `conductor.dispatch.ResourceRequest`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryResourceRequestStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::dispatch::ResourceRequestSnapshot>>>);
impl crate::behaviour::ResourceRequestStorage for MemoryResourceRequestStorage {
fn get(&self, identity: &crate::dispatch::ResourceRequestId) -> Option<crate::dispatch::ResourceRequestSnapshot> { let key = memory_key_164(identity); self.0.borrow().iter().find(|row| memory_key_164(&row.data.resource_request_id) == key).cloned() }
fn put(&mut self, snapshot: crate::dispatch::ResourceRequestSnapshot) { self.delete(&snapshot.data.resource_request_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_164(&row.data.resource_request_id).cmp(&memory_key_164(&other.data.resource_request_id))); }
fn delete(&mut self, identity: &crate::dispatch::ResourceRequestId) { let key = memory_key_164(identity); self.0.borrow_mut().retain(|row| memory_key_164(&row.data.resource_request_id) != key); }
fn list(&self) -> Vec<crate::dispatch::ResourceRequestSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::ResourceRequestStorage for MemoryPorts {
fn get(&self, identity: &crate::dispatch::ResourceRequestId) -> Option<crate::dispatch::ResourceRequestSnapshot> { crate::behaviour::ResourceRequestStorage::get(&self.resource_request_storage, identity) }
fn put(&mut self, snapshot: crate::dispatch::ResourceRequestSnapshot) { crate::behaviour::ResourceRequestStorage::put(&mut self.resource_request_storage, snapshot); }
fn delete(&mut self, identity: &crate::dispatch::ResourceRequestId) { crate::behaviour::ResourceRequestStorage::delete(&mut self.resource_request_storage, identity); }
fn list(&self) -> Vec<crate::dispatch::ResourceRequestSnapshot> { crate::behaviour::ResourceRequestStorage::list(&self.resource_request_storage) }
}

/// Ephemeral storage of `conductor.host.HostGrant`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryHostGrantStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::host::HostGrantSnapshot>>>);
impl crate::behaviour::HostGrantStorage for MemoryHostGrantStorage {
fn get(&self, identity: &crate::host::GrantId) -> Option<crate::host::HostGrantSnapshot> { let key = memory_key_175(identity); self.0.borrow().iter().find(|row| memory_key_175(&row.data.grant_id) == key).cloned() }
fn put(&mut self, snapshot: crate::host::HostGrantSnapshot) { self.delete(&snapshot.data.grant_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_175(&row.data.grant_id).cmp(&memory_key_175(&other.data.grant_id))); }
fn delete(&mut self, identity: &crate::host::GrantId) { let key = memory_key_175(identity); self.0.borrow_mut().retain(|row| memory_key_175(&row.data.grant_id) != key); }
fn list(&self) -> Vec<crate::host::HostGrantSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::HostGrantStorage for MemoryPorts {
fn get(&self, identity: &crate::host::GrantId) -> Option<crate::host::HostGrantSnapshot> { crate::behaviour::HostGrantStorage::get(&self.host_grant_storage, identity) }
fn put(&mut self, snapshot: crate::host::HostGrantSnapshot) { crate::behaviour::HostGrantStorage::put(&mut self.host_grant_storage, snapshot); }
fn delete(&mut self, identity: &crate::host::GrantId) { crate::behaviour::HostGrantStorage::delete(&mut self.host_grant_storage, identity); }
fn list(&self) -> Vec<crate::host::HostGrantSnapshot> { crate::behaviour::HostGrantStorage::list(&self.host_grant_storage) }
}

/// Ephemeral storage of `conductor.host.ReclaimRun`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryReclaimRunStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::host::ReclaimRunSnapshot>>>);
impl crate::behaviour::ReclaimRunStorage for MemoryReclaimRunStorage {
fn get(&self, identity: &crate::host::ReclaimRunId) -> Option<crate::host::ReclaimRunSnapshot> { let key = memory_key_184(identity); self.0.borrow().iter().find(|row| memory_key_184(&row.data.reclaim_run_id) == key).cloned() }
fn put(&mut self, snapshot: crate::host::ReclaimRunSnapshot) { self.delete(&snapshot.data.reclaim_run_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_184(&row.data.reclaim_run_id).cmp(&memory_key_184(&other.data.reclaim_run_id))); }
fn delete(&mut self, identity: &crate::host::ReclaimRunId) { let key = memory_key_184(identity); self.0.borrow_mut().retain(|row| memory_key_184(&row.data.reclaim_run_id) != key); }
fn list(&self) -> Vec<crate::host::ReclaimRunSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::ReclaimRunStorage for MemoryPorts {
fn get(&self, identity: &crate::host::ReclaimRunId) -> Option<crate::host::ReclaimRunSnapshot> { crate::behaviour::ReclaimRunStorage::get(&self.reclaim_run_storage, identity) }
fn put(&mut self, snapshot: crate::host::ReclaimRunSnapshot) { crate::behaviour::ReclaimRunStorage::put(&mut self.reclaim_run_storage, snapshot); }
fn delete(&mut self, identity: &crate::host::ReclaimRunId) { crate::behaviour::ReclaimRunStorage::delete(&mut self.reclaim_run_storage, identity); }
fn list(&self) -> Vec<crate::host::ReclaimRunSnapshot> { crate::behaviour::ReclaimRunStorage::list(&self.reclaim_run_storage) }
}

/// Ephemeral storage of `conductor.improvement.FrictionFinding`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryFrictionFindingStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::improvement::FrictionFindingSnapshot>>>);
impl crate::behaviour::FrictionFindingStorage for MemoryFrictionFindingStorage {
fn get(&self, identity: &crate::improvement::FindingId) -> Option<crate::improvement::FrictionFindingSnapshot> { let key = memory_key_189(identity); self.0.borrow().iter().find(|row| memory_key_189(&row.data.finding_id) == key).cloned() }
fn put(&mut self, snapshot: crate::improvement::FrictionFindingSnapshot) { self.delete(&snapshot.data.finding_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_189(&row.data.finding_id).cmp(&memory_key_189(&other.data.finding_id))); }
fn delete(&mut self, identity: &crate::improvement::FindingId) { let key = memory_key_189(identity); self.0.borrow_mut().retain(|row| memory_key_189(&row.data.finding_id) != key); }
fn list(&self) -> Vec<crate::improvement::FrictionFindingSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::FrictionFindingStorage for MemoryPorts {
fn get(&self, identity: &crate::improvement::FindingId) -> Option<crate::improvement::FrictionFindingSnapshot> { crate::behaviour::FrictionFindingStorage::get(&self.friction_finding_storage, identity) }
fn put(&mut self, snapshot: crate::improvement::FrictionFindingSnapshot) { crate::behaviour::FrictionFindingStorage::put(&mut self.friction_finding_storage, snapshot); }
fn delete(&mut self, identity: &crate::improvement::FindingId) { crate::behaviour::FrictionFindingStorage::delete(&mut self.friction_finding_storage, identity); }
fn list(&self) -> Vec<crate::improvement::FrictionFindingSnapshot> { crate::behaviour::FrictionFindingStorage::list(&self.friction_finding_storage) }
}

/// Ephemeral storage of `conductor.improvement.Metric`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryMetricStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::improvement::MetricSnapshot>>>);
impl crate::behaviour::MetricStorage for MemoryMetricStorage {
fn get(&self, identity: &crate::improvement::MetricId) -> Option<crate::improvement::MetricSnapshot> { let key = memory_key_192(identity); self.0.borrow().iter().find(|row| memory_key_192(&row.data.metric_id) == key).cloned() }
fn put(&mut self, snapshot: crate::improvement::MetricSnapshot) { self.delete(&snapshot.data.metric_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_192(&row.data.metric_id).cmp(&memory_key_192(&other.data.metric_id))); }
fn delete(&mut self, identity: &crate::improvement::MetricId) { let key = memory_key_192(identity); self.0.borrow_mut().retain(|row| memory_key_192(&row.data.metric_id) != key); }
fn list(&self) -> Vec<crate::improvement::MetricSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::MetricStorage for MemoryPorts {
fn get(&self, identity: &crate::improvement::MetricId) -> Option<crate::improvement::MetricSnapshot> { crate::behaviour::MetricStorage::get(&self.metric_storage, identity) }
fn put(&mut self, snapshot: crate::improvement::MetricSnapshot) { crate::behaviour::MetricStorage::put(&mut self.metric_storage, snapshot); }
fn delete(&mut self, identity: &crate::improvement::MetricId) { crate::behaviour::MetricStorage::delete(&mut self.metric_storage, identity); }
fn list(&self) -> Vec<crate::improvement::MetricSnapshot> { crate::behaviour::MetricStorage::list(&self.metric_storage) }
}

/// Ephemeral storage of `conductor.improvement.MetricReading`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryMetricReadingStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::improvement::MetricReadingSnapshot>>>);
impl crate::behaviour::MetricReadingStorage for MemoryMetricReadingStorage {
fn get(&self, identity: &crate::improvement::MetricReadingId) -> Option<crate::improvement::MetricReadingSnapshot> { let key = memory_key_194(identity); self.0.borrow().iter().find(|row| memory_key_194(&row.data.reading_id) == key).cloned() }
fn put(&mut self, snapshot: crate::improvement::MetricReadingSnapshot) { self.delete(&snapshot.data.reading_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_194(&row.data.reading_id).cmp(&memory_key_194(&other.data.reading_id))); }
fn delete(&mut self, identity: &crate::improvement::MetricReadingId) { let key = memory_key_194(identity); self.0.borrow_mut().retain(|row| memory_key_194(&row.data.reading_id) != key); }
fn list(&self) -> Vec<crate::improvement::MetricReadingSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::MetricReadingStorage for MemoryPorts {
fn get(&self, identity: &crate::improvement::MetricReadingId) -> Option<crate::improvement::MetricReadingSnapshot> { crate::behaviour::MetricReadingStorage::get(&self.metric_reading_storage, identity) }
fn put(&mut self, snapshot: crate::improvement::MetricReadingSnapshot) { crate::behaviour::MetricReadingStorage::put(&mut self.metric_reading_storage, snapshot); }
fn delete(&mut self, identity: &crate::improvement::MetricReadingId) { crate::behaviour::MetricReadingStorage::delete(&mut self.metric_reading_storage, identity); }
fn list(&self) -> Vec<crate::improvement::MetricReadingSnapshot> { crate::behaviour::MetricReadingStorage::list(&self.metric_reading_storage) }
}

/// Ephemeral storage of `conductor.improvement.SettingsProposal`, shared by clones.
#[derive(Clone, Default)]
pub struct MemorySettingsProposalStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::improvement::SettingsProposalSnapshot>>>);
impl crate::behaviour::SettingsProposalStorage for MemorySettingsProposalStorage {
fn get(&self, identity: &crate::improvement::ProposalId) -> Option<crate::improvement::SettingsProposalSnapshot> { let key = memory_key_195(identity); self.0.borrow().iter().find(|row| memory_key_195(&row.data.proposal_id) == key).cloned() }
fn put(&mut self, snapshot: crate::improvement::SettingsProposalSnapshot) { self.delete(&snapshot.data.proposal_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_195(&row.data.proposal_id).cmp(&memory_key_195(&other.data.proposal_id))); }
fn delete(&mut self, identity: &crate::improvement::ProposalId) { let key = memory_key_195(identity); self.0.borrow_mut().retain(|row| memory_key_195(&row.data.proposal_id) != key); }
fn list(&self) -> Vec<crate::improvement::SettingsProposalSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::SettingsProposalStorage for MemoryPorts {
fn get(&self, identity: &crate::improvement::ProposalId) -> Option<crate::improvement::SettingsProposalSnapshot> { crate::behaviour::SettingsProposalStorage::get(&self.settings_proposal_storage, identity) }
fn put(&mut self, snapshot: crate::improvement::SettingsProposalSnapshot) { crate::behaviour::SettingsProposalStorage::put(&mut self.settings_proposal_storage, snapshot); }
fn delete(&mut self, identity: &crate::improvement::ProposalId) { crate::behaviour::SettingsProposalStorage::delete(&mut self.settings_proposal_storage, identity); }
fn list(&self) -> Vec<crate::improvement::SettingsProposalSnapshot> { crate::behaviour::SettingsProposalStorage::list(&self.settings_proposal_storage) }
}

/// Ephemeral storage of `conductor.instance.Instance`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryInstanceStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::instance::InstanceSnapshot>>>);
impl crate::behaviour::InstanceStorage for MemoryInstanceStorage {
fn get(&self, identity: &crate::config::InstanceName) -> Option<crate::instance::InstanceSnapshot> { let key = memory_key_49(identity); self.0.borrow().iter().find(|row| memory_key_49(&row.data.name) == key).cloned() }
fn put(&mut self, snapshot: crate::instance::InstanceSnapshot) { self.delete(&snapshot.data.name); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_49(&row.data.name).cmp(&memory_key_49(&other.data.name))); }
fn delete(&mut self, identity: &crate::config::InstanceName) { let key = memory_key_49(identity); self.0.borrow_mut().retain(|row| memory_key_49(&row.data.name) != key); }
fn list(&self) -> Vec<crate::instance::InstanceSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::InstanceStorage for MemoryPorts {
fn get(&self, identity: &crate::config::InstanceName) -> Option<crate::instance::InstanceSnapshot> { crate::behaviour::InstanceStorage::get(&self.instance_storage, identity) }
fn put(&mut self, snapshot: crate::instance::InstanceSnapshot) { crate::behaviour::InstanceStorage::put(&mut self.instance_storage, snapshot); }
fn delete(&mut self, identity: &crate::config::InstanceName) { crate::behaviour::InstanceStorage::delete(&mut self.instance_storage, identity); }
fn list(&self) -> Vec<crate::instance::InstanceSnapshot> { crate::behaviour::InstanceStorage::list(&self.instance_storage) }
}

/// Ephemeral storage of `conductor.instance.Reconciliation`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryReconciliationStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::instance::ReconciliationSnapshot>>>);
impl crate::behaviour::ReconciliationStorage for MemoryReconciliationStorage {
fn get(&self, identity: &crate::instance::ReconciliationId) -> Option<crate::instance::ReconciliationSnapshot> { let key = memory_key_205(identity); self.0.borrow().iter().find(|row| memory_key_205(&row.data.reconciliation_id) == key).cloned() }
fn put(&mut self, snapshot: crate::instance::ReconciliationSnapshot) { self.delete(&snapshot.data.reconciliation_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_205(&row.data.reconciliation_id).cmp(&memory_key_205(&other.data.reconciliation_id))); }
fn delete(&mut self, identity: &crate::instance::ReconciliationId) { let key = memory_key_205(identity); self.0.borrow_mut().retain(|row| memory_key_205(&row.data.reconciliation_id) != key); }
fn list(&self) -> Vec<crate::instance::ReconciliationSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::ReconciliationStorage for MemoryPorts {
fn get(&self, identity: &crate::instance::ReconciliationId) -> Option<crate::instance::ReconciliationSnapshot> { crate::behaviour::ReconciliationStorage::get(&self.reconciliation_storage, identity) }
fn put(&mut self, snapshot: crate::instance::ReconciliationSnapshot) { crate::behaviour::ReconciliationStorage::put(&mut self.reconciliation_storage, snapshot); }
fn delete(&mut self, identity: &crate::instance::ReconciliationId) { crate::behaviour::ReconciliationStorage::delete(&mut self.reconciliation_storage, identity); }
fn list(&self) -> Vec<crate::instance::ReconciliationSnapshot> { crate::behaviour::ReconciliationStorage::list(&self.reconciliation_storage) }
}

/// Ephemeral storage of `conductor.observation.BlockerObservation`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryBlockerObservationStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::observation::BlockerObservationSnapshot>>>);
impl crate::behaviour::BlockerObservationStorage for MemoryBlockerObservationStorage {
fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::BlockerObservationSnapshot> { let key = memory_key_229(identity); self.0.borrow().iter().find(|row| memory_key_229(&row.data.observation_id) == key).cloned() }
fn put(&mut self, snapshot: crate::observation::BlockerObservationSnapshot) { self.delete(&snapshot.data.observation_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_229(&row.data.observation_id).cmp(&memory_key_229(&other.data.observation_id))); }
fn delete(&mut self, identity: &crate::observation::ObservationId) { let key = memory_key_229(identity); self.0.borrow_mut().retain(|row| memory_key_229(&row.data.observation_id) != key); }
fn list(&self) -> Vec<crate::observation::BlockerObservationSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::BlockerObservationStorage for MemoryPorts {
fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::BlockerObservationSnapshot> { crate::behaviour::BlockerObservationStorage::get(&self.blocker_observation_storage, identity) }
fn put(&mut self, snapshot: crate::observation::BlockerObservationSnapshot) { crate::behaviour::BlockerObservationStorage::put(&mut self.blocker_observation_storage, snapshot); }
fn delete(&mut self, identity: &crate::observation::ObservationId) { crate::behaviour::BlockerObservationStorage::delete(&mut self.blocker_observation_storage, identity); }
fn list(&self) -> Vec<crate::observation::BlockerObservationSnapshot> { crate::behaviour::BlockerObservationStorage::list(&self.blocker_observation_storage) }
}

/// Ephemeral storage of `conductor.observation.Board`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryBoardStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::observation::BoardSnapshot>>>);
impl crate::behaviour::BoardStorage for MemoryBoardStorage {
fn get(&self, identity: &crate::observation::BoardId) -> Option<crate::observation::BoardSnapshot> { let key = memory_key_211(identity); self.0.borrow().iter().find(|row| memory_key_211(&row.data.board_id) == key).cloned() }
fn put(&mut self, snapshot: crate::observation::BoardSnapshot) { self.delete(&snapshot.data.board_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_211(&row.data.board_id).cmp(&memory_key_211(&other.data.board_id))); }
fn delete(&mut self, identity: &crate::observation::BoardId) { let key = memory_key_211(identity); self.0.borrow_mut().retain(|row| memory_key_211(&row.data.board_id) != key); }
fn list(&self) -> Vec<crate::observation::BoardSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::BoardStorage for MemoryPorts {
fn get(&self, identity: &crate::observation::BoardId) -> Option<crate::observation::BoardSnapshot> { crate::behaviour::BoardStorage::get(&self.board_storage, identity) }
fn put(&mut self, snapshot: crate::observation::BoardSnapshot) { crate::behaviour::BoardStorage::put(&mut self.board_storage, snapshot); }
fn delete(&mut self, identity: &crate::observation::BoardId) { crate::behaviour::BoardStorage::delete(&mut self.board_storage, identity); }
fn list(&self) -> Vec<crate::observation::BoardSnapshot> { crate::behaviour::BoardStorage::list(&self.board_storage) }
}

/// Ephemeral storage of `conductor.observation.CollectorRunObservation`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryCollectorRunObservationStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::observation::CollectorRunObservationSnapshot>>>);
impl crate::behaviour::CollectorRunObservationStorage for MemoryCollectorRunObservationStorage {
fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::CollectorRunObservationSnapshot> { let key = memory_key_229(identity); self.0.borrow().iter().find(|row| memory_key_229(&row.data.observation_id) == key).cloned() }
fn put(&mut self, snapshot: crate::observation::CollectorRunObservationSnapshot) { self.delete(&snapshot.data.observation_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_229(&row.data.observation_id).cmp(&memory_key_229(&other.data.observation_id))); }
fn delete(&mut self, identity: &crate::observation::ObservationId) { let key = memory_key_229(identity); self.0.borrow_mut().retain(|row| memory_key_229(&row.data.observation_id) != key); }
fn list(&self) -> Vec<crate::observation::CollectorRunObservationSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::CollectorRunObservationStorage for MemoryPorts {
fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::CollectorRunObservationSnapshot> { crate::behaviour::CollectorRunObservationStorage::get(&self.collector_run_observation_storage, identity) }
fn put(&mut self, snapshot: crate::observation::CollectorRunObservationSnapshot) { crate::behaviour::CollectorRunObservationStorage::put(&mut self.collector_run_observation_storage, snapshot); }
fn delete(&mut self, identity: &crate::observation::ObservationId) { crate::behaviour::CollectorRunObservationStorage::delete(&mut self.collector_run_observation_storage, identity); }
fn list(&self) -> Vec<crate::observation::CollectorRunObservationSnapshot> { crate::behaviour::CollectorRunObservationStorage::list(&self.collector_run_observation_storage) }
}

/// Ephemeral storage of `conductor.observation.DailySeries`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryDailySeriesStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::observation::DailySeriesSnapshot>>>);
impl crate::behaviour::DailySeriesStorage for MemoryDailySeriesStorage {
fn get(&self, identity: &crate::observation::DailySeriesId) -> Option<crate::observation::DailySeriesSnapshot> { let key = memory_key_217(identity); self.0.borrow().iter().find(|row| memory_key_217(&row.data.daily_series_id) == key).cloned() }
fn put(&mut self, snapshot: crate::observation::DailySeriesSnapshot) { self.delete(&snapshot.data.daily_series_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_217(&row.data.daily_series_id).cmp(&memory_key_217(&other.data.daily_series_id))); }
fn delete(&mut self, identity: &crate::observation::DailySeriesId) { let key = memory_key_217(identity); self.0.borrow_mut().retain(|row| memory_key_217(&row.data.daily_series_id) != key); }
}
impl crate::behaviour::DailySeriesStorage for MemoryPorts {
fn get(&self, identity: &crate::observation::DailySeriesId) -> Option<crate::observation::DailySeriesSnapshot> { crate::behaviour::DailySeriesStorage::get(&self.daily_series_storage, identity) }
fn put(&mut self, snapshot: crate::observation::DailySeriesSnapshot) { crate::behaviour::DailySeriesStorage::put(&mut self.daily_series_storage, snapshot); }
fn delete(&mut self, identity: &crate::observation::DailySeriesId) { crate::behaviour::DailySeriesStorage::delete(&mut self.daily_series_storage, identity); }
}

/// Ephemeral storage of `conductor.observation.MemberFailureObservation`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryMemberFailureObservationStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::observation::MemberFailureObservationSnapshot>>>);
impl crate::behaviour::MemberFailureObservationStorage for MemoryMemberFailureObservationStorage {
fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::MemberFailureObservationSnapshot> { let key = memory_key_229(identity); self.0.borrow().iter().find(|row| memory_key_229(&row.data.observation_id) == key).cloned() }
fn put(&mut self, snapshot: crate::observation::MemberFailureObservationSnapshot) { self.delete(&snapshot.data.observation_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_229(&row.data.observation_id).cmp(&memory_key_229(&other.data.observation_id))); }
fn delete(&mut self, identity: &crate::observation::ObservationId) { let key = memory_key_229(identity); self.0.borrow_mut().retain(|row| memory_key_229(&row.data.observation_id) != key); }
fn list(&self) -> Vec<crate::observation::MemberFailureObservationSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::MemberFailureObservationStorage for MemoryPorts {
fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::MemberFailureObservationSnapshot> { crate::behaviour::MemberFailureObservationStorage::get(&self.member_failure_observation_storage, identity) }
fn put(&mut self, snapshot: crate::observation::MemberFailureObservationSnapshot) { crate::behaviour::MemberFailureObservationStorage::put(&mut self.member_failure_observation_storage, snapshot); }
fn delete(&mut self, identity: &crate::observation::ObservationId) { crate::behaviour::MemberFailureObservationStorage::delete(&mut self.member_failure_observation_storage, identity); }
fn list(&self) -> Vec<crate::observation::MemberFailureObservationSnapshot> { crate::behaviour::MemberFailureObservationStorage::list(&self.member_failure_observation_storage) }
}

/// Ephemeral storage of `conductor.observation.MergedPullRequestObservation`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryMergedPullRequestObservationStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::observation::MergedPullRequestObservationSnapshot>>>);
impl crate::behaviour::MergedPullRequestObservationStorage for MemoryMergedPullRequestObservationStorage {
fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::MergedPullRequestObservationSnapshot> { let key = memory_key_229(identity); self.0.borrow().iter().find(|row| memory_key_229(&row.data.observation_id) == key).cloned() }
fn put(&mut self, snapshot: crate::observation::MergedPullRequestObservationSnapshot) { self.delete(&snapshot.data.observation_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_229(&row.data.observation_id).cmp(&memory_key_229(&other.data.observation_id))); }
fn delete(&mut self, identity: &crate::observation::ObservationId) { let key = memory_key_229(identity); self.0.borrow_mut().retain(|row| memory_key_229(&row.data.observation_id) != key); }
fn list(&self) -> Vec<crate::observation::MergedPullRequestObservationSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::MergedPullRequestObservationStorage for MemoryPorts {
fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::MergedPullRequestObservationSnapshot> { crate::behaviour::MergedPullRequestObservationStorage::get(&self.merged_pull_request_observation_storage, identity) }
fn put(&mut self, snapshot: crate::observation::MergedPullRequestObservationSnapshot) { crate::behaviour::MergedPullRequestObservationStorage::put(&mut self.merged_pull_request_observation_storage, snapshot); }
fn delete(&mut self, identity: &crate::observation::ObservationId) { crate::behaviour::MergedPullRequestObservationStorage::delete(&mut self.merged_pull_request_observation_storage, identity); }
fn list(&self) -> Vec<crate::observation::MergedPullRequestObservationSnapshot> { crate::behaviour::MergedPullRequestObservationStorage::list(&self.merged_pull_request_observation_storage) }
}

/// Ephemeral storage of `conductor.observation.PullRequestObservation`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryPullRequestObservationStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::observation::PullRequestObservationSnapshot>>>);
impl crate::behaviour::PullRequestObservationStorage for MemoryPullRequestObservationStorage {
fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::PullRequestObservationSnapshot> { let key = memory_key_229(identity); self.0.borrow().iter().find(|row| memory_key_229(&row.data.observation_id) == key).cloned() }
fn put(&mut self, snapshot: crate::observation::PullRequestObservationSnapshot) { self.delete(&snapshot.data.observation_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_229(&row.data.observation_id).cmp(&memory_key_229(&other.data.observation_id))); }
fn delete(&mut self, identity: &crate::observation::ObservationId) { let key = memory_key_229(identity); self.0.borrow_mut().retain(|row| memory_key_229(&row.data.observation_id) != key); }
fn list(&self) -> Vec<crate::observation::PullRequestObservationSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::PullRequestObservationStorage for MemoryPorts {
fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::PullRequestObservationSnapshot> { crate::behaviour::PullRequestObservationStorage::get(&self.pull_request_observation_storage, identity) }
fn put(&mut self, snapshot: crate::observation::PullRequestObservationSnapshot) { crate::behaviour::PullRequestObservationStorage::put(&mut self.pull_request_observation_storage, snapshot); }
fn delete(&mut self, identity: &crate::observation::ObservationId) { crate::behaviour::PullRequestObservationStorage::delete(&mut self.pull_request_observation_storage, identity); }
fn list(&self) -> Vec<crate::observation::PullRequestObservationSnapshot> { crate::behaviour::PullRequestObservationStorage::list(&self.pull_request_observation_storage) }
}

/// Ephemeral storage of `conductor.observation.ReleaseObservation`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryReleaseObservationStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::observation::ReleaseObservationSnapshot>>>);
impl crate::behaviour::ReleaseObservationStorage for MemoryReleaseObservationStorage {
fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::ReleaseObservationSnapshot> { let key = memory_key_229(identity); self.0.borrow().iter().find(|row| memory_key_229(&row.data.observation_id) == key).cloned() }
fn put(&mut self, snapshot: crate::observation::ReleaseObservationSnapshot) { self.delete(&snapshot.data.observation_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_229(&row.data.observation_id).cmp(&memory_key_229(&other.data.observation_id))); }
fn delete(&mut self, identity: &crate::observation::ObservationId) { let key = memory_key_229(identity); self.0.borrow_mut().retain(|row| memory_key_229(&row.data.observation_id) != key); }
fn list(&self) -> Vec<crate::observation::ReleaseObservationSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::ReleaseObservationStorage for MemoryPorts {
fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::ReleaseObservationSnapshot> { crate::behaviour::ReleaseObservationStorage::get(&self.release_observation_storage, identity) }
fn put(&mut self, snapshot: crate::observation::ReleaseObservationSnapshot) { crate::behaviour::ReleaseObservationStorage::put(&mut self.release_observation_storage, snapshot); }
fn delete(&mut self, identity: &crate::observation::ObservationId) { crate::behaviour::ReleaseObservationStorage::delete(&mut self.release_observation_storage, identity); }
fn list(&self) -> Vec<crate::observation::ReleaseObservationSnapshot> { crate::behaviour::ReleaseObservationStorage::list(&self.release_observation_storage) }
}

/// Ephemeral storage of `conductor.observation.RepositoryObservation`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryRepositoryObservationStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::observation::RepositoryObservationSnapshot>>>);
impl crate::behaviour::RepositoryObservationStorage for MemoryRepositoryObservationStorage {
fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::RepositoryObservationSnapshot> { let key = memory_key_229(identity); self.0.borrow().iter().find(|row| memory_key_229(&row.data.observation_id) == key).cloned() }
fn put(&mut self, snapshot: crate::observation::RepositoryObservationSnapshot) { self.delete(&snapshot.data.observation_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_229(&row.data.observation_id).cmp(&memory_key_229(&other.data.observation_id))); }
fn delete(&mut self, identity: &crate::observation::ObservationId) { let key = memory_key_229(identity); self.0.borrow_mut().retain(|row| memory_key_229(&row.data.observation_id) != key); }
fn list(&self) -> Vec<crate::observation::RepositoryObservationSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::RepositoryObservationStorage for MemoryPorts {
fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::RepositoryObservationSnapshot> { crate::behaviour::RepositoryObservationStorage::get(&self.repository_observation_storage, identity) }
fn put(&mut self, snapshot: crate::observation::RepositoryObservationSnapshot) { crate::behaviour::RepositoryObservationStorage::put(&mut self.repository_observation_storage, snapshot); }
fn delete(&mut self, identity: &crate::observation::ObservationId) { crate::behaviour::RepositoryObservationStorage::delete(&mut self.repository_observation_storage, identity); }
fn list(&self) -> Vec<crate::observation::RepositoryObservationSnapshot> { crate::behaviour::RepositoryObservationStorage::list(&self.repository_observation_storage) }
}

/// Ephemeral storage of `conductor.observation.SecretSyncObservation`, shared by clones.
#[derive(Clone, Default)]
pub struct MemorySecretSyncObservationStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::observation::SecretSyncObservationSnapshot>>>);
impl crate::behaviour::SecretSyncObservationStorage for MemorySecretSyncObservationStorage {
fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::SecretSyncObservationSnapshot> { let key = memory_key_229(identity); self.0.borrow().iter().find(|row| memory_key_229(&row.data.observation_id) == key).cloned() }
fn put(&mut self, snapshot: crate::observation::SecretSyncObservationSnapshot) { self.delete(&snapshot.data.observation_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_229(&row.data.observation_id).cmp(&memory_key_229(&other.data.observation_id))); }
fn delete(&mut self, identity: &crate::observation::ObservationId) { let key = memory_key_229(identity); self.0.borrow_mut().retain(|row| memory_key_229(&row.data.observation_id) != key); }
fn list(&self) -> Vec<crate::observation::SecretSyncObservationSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::SecretSyncObservationStorage for MemoryPorts {
fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::SecretSyncObservationSnapshot> { crate::behaviour::SecretSyncObservationStorage::get(&self.secret_sync_observation_storage, identity) }
fn put(&mut self, snapshot: crate::observation::SecretSyncObservationSnapshot) { crate::behaviour::SecretSyncObservationStorage::put(&mut self.secret_sync_observation_storage, snapshot); }
fn delete(&mut self, identity: &crate::observation::ObservationId) { crate::behaviour::SecretSyncObservationStorage::delete(&mut self.secret_sync_observation_storage, identity); }
fn list(&self) -> Vec<crate::observation::SecretSyncObservationSnapshot> { crate::behaviour::SecretSyncObservationStorage::list(&self.secret_sync_observation_storage) }
}

/// Ephemeral storage of `conductor.observation.SessionObservation`, shared by clones.
#[derive(Clone, Default)]
pub struct MemorySessionObservationStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::observation::SessionObservationSnapshot>>>);
impl crate::behaviour::SessionObservationStorage for MemorySessionObservationStorage {
fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::SessionObservationSnapshot> { let key = memory_key_229(identity); self.0.borrow().iter().find(|row| memory_key_229(&row.data.observation_id) == key).cloned() }
fn put(&mut self, snapshot: crate::observation::SessionObservationSnapshot) { self.delete(&snapshot.data.observation_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_229(&row.data.observation_id).cmp(&memory_key_229(&other.data.observation_id))); }
fn delete(&mut self, identity: &crate::observation::ObservationId) { let key = memory_key_229(identity); self.0.borrow_mut().retain(|row| memory_key_229(&row.data.observation_id) != key); }
fn list(&self) -> Vec<crate::observation::SessionObservationSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::SessionObservationStorage for MemoryPorts {
fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::SessionObservationSnapshot> { crate::behaviour::SessionObservationStorage::get(&self.session_observation_storage, identity) }
fn put(&mut self, snapshot: crate::observation::SessionObservationSnapshot) { crate::behaviour::SessionObservationStorage::put(&mut self.session_observation_storage, snapshot); }
fn delete(&mut self, identity: &crate::observation::ObservationId) { crate::behaviour::SessionObservationStorage::delete(&mut self.session_observation_storage, identity); }
fn list(&self) -> Vec<crate::observation::SessionObservationSnapshot> { crate::behaviour::SessionObservationStorage::list(&self.session_observation_storage) }
}

/// Ephemeral storage of `conductor.observation.Snapshot`, shared by clones.
#[derive(Clone, Default)]
pub struct MemorySnapshotStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::observation::SnapshotSnapshot>>>);
impl crate::behaviour::SnapshotStorage for MemorySnapshotStorage {
fn get(&self, identity: &crate::observation::SnapshotId) -> Option<crate::observation::SnapshotSnapshot> { let key = memory_key_243(identity); self.0.borrow().iter().find(|row| memory_key_243(&row.data.snapshot_id) == key).cloned() }
fn put(&mut self, snapshot: crate::observation::SnapshotSnapshot) { self.delete(&snapshot.data.snapshot_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_243(&row.data.snapshot_id).cmp(&memory_key_243(&other.data.snapshot_id))); }
fn delete(&mut self, identity: &crate::observation::SnapshotId) { let key = memory_key_243(identity); self.0.borrow_mut().retain(|row| memory_key_243(&row.data.snapshot_id) != key); }
fn list(&self) -> Vec<crate::observation::SnapshotSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::SnapshotStorage for MemoryPorts {
fn get(&self, identity: &crate::observation::SnapshotId) -> Option<crate::observation::SnapshotSnapshot> { crate::behaviour::SnapshotStorage::get(&self.snapshot_storage, identity) }
fn put(&mut self, snapshot: crate::observation::SnapshotSnapshot) { crate::behaviour::SnapshotStorage::put(&mut self.snapshot_storage, snapshot); }
fn delete(&mut self, identity: &crate::observation::SnapshotId) { crate::behaviour::SnapshotStorage::delete(&mut self.snapshot_storage, identity); }
fn list(&self) -> Vec<crate::observation::SnapshotSnapshot> { crate::behaviour::SnapshotStorage::list(&self.snapshot_storage) }
}

/// Ephemeral storage of `conductor.observation.SpecificationObservation`, shared by clones.
#[derive(Clone, Default)]
pub struct MemorySpecificationObservationStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::observation::SpecificationObservationSnapshot>>>);
impl crate::behaviour::SpecificationObservationStorage for MemorySpecificationObservationStorage {
fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::SpecificationObservationSnapshot> { let key = memory_key_229(identity); self.0.borrow().iter().find(|row| memory_key_229(&row.data.observation_id) == key).cloned() }
fn put(&mut self, snapshot: crate::observation::SpecificationObservationSnapshot) { self.delete(&snapshot.data.observation_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_229(&row.data.observation_id).cmp(&memory_key_229(&other.data.observation_id))); }
fn delete(&mut self, identity: &crate::observation::ObservationId) { let key = memory_key_229(identity); self.0.borrow_mut().retain(|row| memory_key_229(&row.data.observation_id) != key); }
fn list(&self) -> Vec<crate::observation::SpecificationObservationSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::SpecificationObservationStorage for MemoryPorts {
fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::SpecificationObservationSnapshot> { crate::behaviour::SpecificationObservationStorage::get(&self.specification_observation_storage, identity) }
fn put(&mut self, snapshot: crate::observation::SpecificationObservationSnapshot) { crate::behaviour::SpecificationObservationStorage::put(&mut self.specification_observation_storage, snapshot); }
fn delete(&mut self, identity: &crate::observation::ObservationId) { crate::behaviour::SpecificationObservationStorage::delete(&mut self.specification_observation_storage, identity); }
fn list(&self) -> Vec<crate::observation::SpecificationObservationSnapshot> { crate::behaviour::SpecificationObservationStorage::list(&self.specification_observation_storage) }
}

/// Ephemeral storage of `conductor.observation.UsageObservation`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryUsageObservationStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::observation::UsageObservationSnapshot>>>);
impl crate::behaviour::UsageObservationStorage for MemoryUsageObservationStorage {
fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::UsageObservationSnapshot> { let key = memory_key_229(identity); self.0.borrow().iter().find(|row| memory_key_229(&row.data.observation_id) == key).cloned() }
fn put(&mut self, snapshot: crate::observation::UsageObservationSnapshot) { self.delete(&snapshot.data.observation_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_229(&row.data.observation_id).cmp(&memory_key_229(&other.data.observation_id))); }
fn delete(&mut self, identity: &crate::observation::ObservationId) { let key = memory_key_229(identity); self.0.borrow_mut().retain(|row| memory_key_229(&row.data.observation_id) != key); }
fn list(&self) -> Vec<crate::observation::UsageObservationSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::UsageObservationStorage for MemoryPorts {
fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::UsageObservationSnapshot> { crate::behaviour::UsageObservationStorage::get(&self.usage_observation_storage, identity) }
fn put(&mut self, snapshot: crate::observation::UsageObservationSnapshot) { crate::behaviour::UsageObservationStorage::put(&mut self.usage_observation_storage, snapshot); }
fn delete(&mut self, identity: &crate::observation::ObservationId) { crate::behaviour::UsageObservationStorage::delete(&mut self.usage_observation_storage, identity); }
fn list(&self) -> Vec<crate::observation::UsageObservationSnapshot> { crate::behaviour::UsageObservationStorage::list(&self.usage_observation_storage) }
}

/// Ephemeral storage of `conductor.observation.WorkflowRunObservation`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryWorkflowRunObservationStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::observation::WorkflowRunObservationSnapshot>>>);
impl crate::behaviour::WorkflowRunObservationStorage for MemoryWorkflowRunObservationStorage {
fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::WorkflowRunObservationSnapshot> { let key = memory_key_229(identity); self.0.borrow().iter().find(|row| memory_key_229(&row.data.observation_id) == key).cloned() }
fn put(&mut self, snapshot: crate::observation::WorkflowRunObservationSnapshot) { self.delete(&snapshot.data.observation_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_229(&row.data.observation_id).cmp(&memory_key_229(&other.data.observation_id))); }
fn delete(&mut self, identity: &crate::observation::ObservationId) { let key = memory_key_229(identity); self.0.borrow_mut().retain(|row| memory_key_229(&row.data.observation_id) != key); }
fn list(&self) -> Vec<crate::observation::WorkflowRunObservationSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::WorkflowRunObservationStorage for MemoryPorts {
fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::WorkflowRunObservationSnapshot> { crate::behaviour::WorkflowRunObservationStorage::get(&self.workflow_run_observation_storage, identity) }
fn put(&mut self, snapshot: crate::observation::WorkflowRunObservationSnapshot) { crate::behaviour::WorkflowRunObservationStorage::put(&mut self.workflow_run_observation_storage, snapshot); }
fn delete(&mut self, identity: &crate::observation::ObservationId) { crate::behaviour::WorkflowRunObservationStorage::delete(&mut self.workflow_run_observation_storage, identity); }
fn list(&self) -> Vec<crate::observation::WorkflowRunObservationSnapshot> { crate::behaviour::WorkflowRunObservationStorage::list(&self.workflow_run_observation_storage) }
}

/// Ephemeral storage of `conductor.operator.Operator`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryOperatorStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::operator::OperatorSnapshot>>>);
impl crate::behaviour::OperatorStorage for MemoryOperatorStorage {
fn get(&self, identity: &crate::operator::OperatorKey) -> Option<crate::operator::OperatorSnapshot> { let key = memory_key_263(identity); self.0.borrow().iter().find(|row| memory_key_263(&row.data.key) == key).cloned() }
fn put(&mut self, snapshot: crate::operator::OperatorSnapshot) { self.delete(&snapshot.data.key); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_263(&row.data.key).cmp(&memory_key_263(&other.data.key))); }
fn delete(&mut self, identity: &crate::operator::OperatorKey) { let key = memory_key_263(identity); self.0.borrow_mut().retain(|row| memory_key_263(&row.data.key) != key); }
fn list(&self) -> Vec<crate::operator::OperatorSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::OperatorStorage for MemoryPorts {
fn get(&self, identity: &crate::operator::OperatorKey) -> Option<crate::operator::OperatorSnapshot> { crate::behaviour::OperatorStorage::get(&self.operator_storage, identity) }
fn put(&mut self, snapshot: crate::operator::OperatorSnapshot) { crate::behaviour::OperatorStorage::put(&mut self.operator_storage, snapshot); }
fn delete(&mut self, identity: &crate::operator::OperatorKey) { crate::behaviour::OperatorStorage::delete(&mut self.operator_storage, identity); }
fn list(&self) -> Vec<crate::operator::OperatorSnapshot> { crate::behaviour::OperatorStorage::list(&self.operator_storage) }
}

/// Ephemeral storage of `conductor.placement.Box`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryBoxStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::placement::BoxSnapshot>>>);
impl crate::behaviour::BoxStorage for MemoryBoxStorage {
fn get(&self, identity: &crate::placement::BoxId) -> Option<crate::placement::BoxSnapshot> { let key = memory_key_267(identity); self.0.borrow().iter().find(|row| memory_key_267(&row.data.box_id) == key).cloned() }
fn put(&mut self, snapshot: crate::placement::BoxSnapshot) { self.delete(&snapshot.data.box_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_267(&row.data.box_id).cmp(&memory_key_267(&other.data.box_id))); }
fn delete(&mut self, identity: &crate::placement::BoxId) { let key = memory_key_267(identity); self.0.borrow_mut().retain(|row| memory_key_267(&row.data.box_id) != key); }
fn list(&self) -> Vec<crate::placement::BoxSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::BoxStorage for MemoryPorts {
fn get(&self, identity: &crate::placement::BoxId) -> Option<crate::placement::BoxSnapshot> { crate::behaviour::BoxStorage::get(&self.box_storage, identity) }
fn put(&mut self, snapshot: crate::placement::BoxSnapshot) { crate::behaviour::BoxStorage::put(&mut self.box_storage, snapshot); }
fn delete(&mut self, identity: &crate::placement::BoxId) { crate::behaviour::BoxStorage::delete(&mut self.box_storage, identity); }
fn list(&self) -> Vec<crate::placement::BoxSnapshot> { crate::behaviour::BoxStorage::list(&self.box_storage) }
}

/// Ephemeral storage of `conductor.placement.BoxRefusal`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryBoxRefusalStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::placement::BoxRefusalSnapshot>>>);
impl crate::behaviour::BoxRefusalStorage for MemoryBoxRefusalStorage {
fn get(&self, identity: &crate::placement::BoxId) -> Option<crate::placement::BoxRefusalSnapshot> { let key = memory_key_267(identity); self.0.borrow().iter().find(|row| memory_key_267(&row.data.box_id) == key).cloned() }
fn put(&mut self, snapshot: crate::placement::BoxRefusalSnapshot) { self.delete(&snapshot.data.box_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_267(&row.data.box_id).cmp(&memory_key_267(&other.data.box_id))); }
fn delete(&mut self, identity: &crate::placement::BoxId) { let key = memory_key_267(identity); self.0.borrow_mut().retain(|row| memory_key_267(&row.data.box_id) != key); }
fn list(&self) -> Vec<crate::placement::BoxRefusalSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::BoxRefusalStorage for MemoryPorts {
fn get(&self, identity: &crate::placement::BoxId) -> Option<crate::placement::BoxRefusalSnapshot> { crate::behaviour::BoxRefusalStorage::get(&self.box_refusal_storage, identity) }
fn put(&mut self, snapshot: crate::placement::BoxRefusalSnapshot) { crate::behaviour::BoxRefusalStorage::put(&mut self.box_refusal_storage, snapshot); }
fn delete(&mut self, identity: &crate::placement::BoxId) { crate::behaviour::BoxRefusalStorage::delete(&mut self.box_refusal_storage, identity); }
fn list(&self) -> Vec<crate::placement::BoxRefusalSnapshot> { crate::behaviour::BoxRefusalStorage::list(&self.box_refusal_storage) }
}

/// Ephemeral storage of `conductor.placement.Placement`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryPlacementStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::placement::PlacementSnapshot>>>);
impl crate::behaviour::PlacementStorage for MemoryPlacementStorage {
fn get(&self, identity: &crate::placement::PlacementName) -> Option<crate::placement::PlacementSnapshot> { let key = memory_key_275(identity); self.0.borrow().iter().find(|row| memory_key_275(&row.data.name) == key).cloned() }
fn put(&mut self, snapshot: crate::placement::PlacementSnapshot) { self.delete(&snapshot.data.name); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_275(&row.data.name).cmp(&memory_key_275(&other.data.name))); }
fn delete(&mut self, identity: &crate::placement::PlacementName) { let key = memory_key_275(identity); self.0.borrow_mut().retain(|row| memory_key_275(&row.data.name) != key); }
fn list(&self) -> Vec<crate::placement::PlacementSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::PlacementStorage for MemoryPorts {
fn get(&self, identity: &crate::placement::PlacementName) -> Option<crate::placement::PlacementSnapshot> { crate::behaviour::PlacementStorage::get(&self.placement_storage, identity) }
fn put(&mut self, snapshot: crate::placement::PlacementSnapshot) { crate::behaviour::PlacementStorage::put(&mut self.placement_storage, snapshot); }
fn delete(&mut self, identity: &crate::placement::PlacementName) { crate::behaviour::PlacementStorage::delete(&mut self.placement_storage, identity); }
fn list(&self) -> Vec<crate::placement::PlacementSnapshot> { crate::behaviour::PlacementStorage::list(&self.placement_storage) }
}

/// Ephemeral storage of `conductor.secret.Secret`, shared by clones.
#[derive(Clone, Default)]
pub struct MemorySecretStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::secret::SecretSnapshot>>>);
impl crate::behaviour::SecretStorage for MemorySecretStorage {
fn get(&self, identity: &crate::secret::SecretName) -> Option<crate::secret::SecretSnapshot> { let key = memory_key_280(identity); self.0.borrow().iter().find(|row| memory_key_280(&row.data.name) == key).cloned() }
fn put(&mut self, snapshot: crate::secret::SecretSnapshot) { self.delete(&snapshot.data.name); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_280(&row.data.name).cmp(&memory_key_280(&other.data.name))); }
fn delete(&mut self, identity: &crate::secret::SecretName) { let key = memory_key_280(identity); self.0.borrow_mut().retain(|row| memory_key_280(&row.data.name) != key); }
fn list(&self) -> Vec<crate::secret::SecretSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::SecretStorage for MemoryPorts {
fn get(&self, identity: &crate::secret::SecretName) -> Option<crate::secret::SecretSnapshot> { crate::behaviour::SecretStorage::get(&self.secret_storage, identity) }
fn put(&mut self, snapshot: crate::secret::SecretSnapshot) { crate::behaviour::SecretStorage::put(&mut self.secret_storage, snapshot); }
fn delete(&mut self, identity: &crate::secret::SecretName) { crate::behaviour::SecretStorage::delete(&mut self.secret_storage, identity); }
fn list(&self) -> Vec<crate::secret::SecretSnapshot> { crate::behaviour::SecretStorage::list(&self.secret_storage) }
}

/// Ephemeral storage of `conductor.session.Delivery`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryDeliveryStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::session::DeliverySnapshot>>>);
impl crate::behaviour::DeliveryStorage for MemoryDeliveryStorage {
fn get(&self, identity: &crate::session::DeliveryId) -> Option<crate::session::DeliverySnapshot> { let key = memory_key_290(identity); self.0.borrow().iter().find(|row| memory_key_290(&row.data.delivery_id) == key).cloned() }
fn put(&mut self, snapshot: crate::session::DeliverySnapshot) { self.delete(&snapshot.data.delivery_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_290(&row.data.delivery_id).cmp(&memory_key_290(&other.data.delivery_id))); }
fn delete(&mut self, identity: &crate::session::DeliveryId) { let key = memory_key_290(identity); self.0.borrow_mut().retain(|row| memory_key_290(&row.data.delivery_id) != key); }
fn list(&self) -> Vec<crate::session::DeliverySnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::DeliveryStorage for MemoryPorts {
fn get(&self, identity: &crate::session::DeliveryId) -> Option<crate::session::DeliverySnapshot> { crate::behaviour::DeliveryStorage::get(&self.delivery_storage, identity) }
fn put(&mut self, snapshot: crate::session::DeliverySnapshot) { crate::behaviour::DeliveryStorage::put(&mut self.delivery_storage, snapshot); }
fn delete(&mut self, identity: &crate::session::DeliveryId) { crate::behaviour::DeliveryStorage::delete(&mut self.delivery_storage, identity); }
fn list(&self) -> Vec<crate::session::DeliverySnapshot> { crate::behaviour::DeliveryStorage::list(&self.delivery_storage) }
}

/// Ephemeral storage of `conductor.session.Lease`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryLeaseStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::session::LeaseSnapshot>>>);
impl crate::behaviour::LeaseStorage for MemoryLeaseStorage {
fn get(&self, identity: &crate::config::InstanceName) -> Option<crate::session::LeaseSnapshot> { let key = memory_key_49(identity); self.0.borrow().iter().find(|row| memory_key_49(&row.data.instance) == key).cloned() }
fn put(&mut self, snapshot: crate::session::LeaseSnapshot) { self.delete(&snapshot.data.instance); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_49(&row.data.instance).cmp(&memory_key_49(&other.data.instance))); }
fn delete(&mut self, identity: &crate::config::InstanceName) { let key = memory_key_49(identity); self.0.borrow_mut().retain(|row| memory_key_49(&row.data.instance) != key); }
fn list(&self) -> Vec<crate::session::LeaseSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::LeaseStorage for MemoryPorts {
fn get(&self, identity: &crate::config::InstanceName) -> Option<crate::session::LeaseSnapshot> { crate::behaviour::LeaseStorage::get(&self.lease_storage, identity) }
fn put(&mut self, snapshot: crate::session::LeaseSnapshot) { crate::behaviour::LeaseStorage::put(&mut self.lease_storage, snapshot); }
fn delete(&mut self, identity: &crate::config::InstanceName) { crate::behaviour::LeaseStorage::delete(&mut self.lease_storage, identity); }
fn list(&self) -> Vec<crate::session::LeaseSnapshot> { crate::behaviour::LeaseStorage::list(&self.lease_storage) }
}

/// Ephemeral storage of `conductor.session.RunTick`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryRunTickStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::session::RunTickSnapshot>>>);
impl crate::behaviour::RunTickStorage for MemoryRunTickStorage {
fn get(&self, identity: &crate::session::TickId) -> Option<crate::session::RunTickSnapshot> { let key = memory_key_308(identity); self.0.borrow().iter().find(|row| memory_key_308(&row.data.tick_id) == key).cloned() }
fn put(&mut self, snapshot: crate::session::RunTickSnapshot) { self.delete(&snapshot.data.tick_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_308(&row.data.tick_id).cmp(&memory_key_308(&other.data.tick_id))); }
fn delete(&mut self, identity: &crate::session::TickId) { let key = memory_key_308(identity); self.0.borrow_mut().retain(|row| memory_key_308(&row.data.tick_id) != key); }
fn list(&self) -> Vec<crate::session::RunTickSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::RunTickStorage for MemoryPorts {
fn get(&self, identity: &crate::session::TickId) -> Option<crate::session::RunTickSnapshot> { crate::behaviour::RunTickStorage::get(&self.run_tick_storage, identity) }
fn put(&mut self, snapshot: crate::session::RunTickSnapshot) { crate::behaviour::RunTickStorage::put(&mut self.run_tick_storage, snapshot); }
fn delete(&mut self, identity: &crate::session::TickId) { crate::behaviour::RunTickStorage::delete(&mut self.run_tick_storage, identity); }
fn list(&self) -> Vec<crate::session::RunTickSnapshot> { crate::behaviour::RunTickStorage::list(&self.run_tick_storage) }
}

/// Ephemeral storage of `conductor.store.Build`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryBuildStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::store::BuildSnapshot>>>);
impl crate::behaviour::BuildStorage for MemoryBuildStorage {
fn get(&self, identity: &crate::store::BinaryVersion) -> Option<crate::store::BuildSnapshot> { let key = memory_key_311(identity); self.0.borrow().iter().find(|row| memory_key_311(&row.data.version) == key).cloned() }
fn put(&mut self, snapshot: crate::store::BuildSnapshot) { self.delete(&snapshot.data.version); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_311(&row.data.version).cmp(&memory_key_311(&other.data.version))); }
fn delete(&mut self, identity: &crate::store::BinaryVersion) { let key = memory_key_311(identity); self.0.borrow_mut().retain(|row| memory_key_311(&row.data.version) != key); }
fn list(&self) -> Vec<crate::store::BuildSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::BuildStorage for MemoryPorts {
fn get(&self, identity: &crate::store::BinaryVersion) -> Option<crate::store::BuildSnapshot> { crate::behaviour::BuildStorage::get(&self.build_storage, identity) }
fn put(&mut self, snapshot: crate::store::BuildSnapshot) { crate::behaviour::BuildStorage::put(&mut self.build_storage, snapshot); }
fn delete(&mut self, identity: &crate::store::BinaryVersion) { crate::behaviour::BuildStorage::delete(&mut self.build_storage, identity); }
fn list(&self) -> Vec<crate::store::BuildSnapshot> { crate::behaviour::BuildStorage::list(&self.build_storage) }
}

/// Ephemeral storage of `conductor.store.Migration`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryMigrationStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::store::MigrationSnapshot>>>);
impl crate::behaviour::MigrationStorage for MemoryMigrationStorage {
fn get(&self, identity: &crate::config::InstanceName) -> Option<crate::store::MigrationSnapshot> { let key = memory_key_49(identity); self.0.borrow().iter().find(|row| memory_key_49(&row.data.instance) == key).cloned() }
fn put(&mut self, snapshot: crate::store::MigrationSnapshot) { self.delete(&snapshot.data.instance); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_49(&row.data.instance).cmp(&memory_key_49(&other.data.instance))); }
fn delete(&mut self, identity: &crate::config::InstanceName) { let key = memory_key_49(identity); self.0.borrow_mut().retain(|row| memory_key_49(&row.data.instance) != key); }
fn list(&self) -> Vec<crate::store::MigrationSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::MigrationStorage for MemoryPorts {
fn get(&self, identity: &crate::config::InstanceName) -> Option<crate::store::MigrationSnapshot> { crate::behaviour::MigrationStorage::get(&self.migration_storage, identity) }
fn put(&mut self, snapshot: crate::store::MigrationSnapshot) { crate::behaviour::MigrationStorage::put(&mut self.migration_storage, snapshot); }
fn delete(&mut self, identity: &crate::config::InstanceName) { crate::behaviour::MigrationStorage::delete(&mut self.migration_storage, identity); }
fn list(&self) -> Vec<crate::store::MigrationSnapshot> { crate::behaviour::MigrationStorage::list(&self.migration_storage) }
}

/// Ephemeral storage of `conductor.store.Store`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryStoreStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::store::StoreSnapshot>>>);
impl crate::behaviour::StoreStorage for MemoryStoreStorage {
fn get(&self, identity: &crate::store::StoreUrl) -> Option<crate::store::StoreSnapshot> { let key = memory_key_315(identity); self.0.borrow().iter().find(|row| memory_key_315(&row.data.url) == key).cloned() }
fn put(&mut self, snapshot: crate::store::StoreSnapshot) { self.delete(&snapshot.data.url); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_315(&row.data.url).cmp(&memory_key_315(&other.data.url))); }
fn delete(&mut self, identity: &crate::store::StoreUrl) { let key = memory_key_315(identity); self.0.borrow_mut().retain(|row| memory_key_315(&row.data.url) != key); }
fn list(&self) -> Vec<crate::store::StoreSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::StoreStorage for MemoryPorts {
fn get(&self, identity: &crate::store::StoreUrl) -> Option<crate::store::StoreSnapshot> { crate::behaviour::StoreStorage::get(&self.store_storage, identity) }
fn put(&mut self, snapshot: crate::store::StoreSnapshot) { crate::behaviour::StoreStorage::put(&mut self.store_storage, snapshot); }
fn delete(&mut self, identity: &crate::store::StoreUrl) { crate::behaviour::StoreStorage::delete(&mut self.store_storage, identity); }
fn list(&self) -> Vec<crate::store::StoreSnapshot> { crate::behaviour::StoreStorage::list(&self.store_storage) }
}

/// Ephemeral storage of `conductor.token.PoolToken`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryPoolTokenStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::token::PoolTokenSnapshot>>>);
impl crate::behaviour::PoolTokenStorage for MemoryPoolTokenStorage {
fn get(&self, identity: &crate::token::TokenId) -> Option<crate::token::PoolTokenSnapshot> { let key = memory_key_321(identity); self.0.borrow().iter().find(|row| memory_key_321(&row.data.token_id) == key).cloned() }
fn put(&mut self, snapshot: crate::token::PoolTokenSnapshot) { self.delete(&snapshot.data.token_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_321(&row.data.token_id).cmp(&memory_key_321(&other.data.token_id))); }
fn delete(&mut self, identity: &crate::token::TokenId) { let key = memory_key_321(identity); self.0.borrow_mut().retain(|row| memory_key_321(&row.data.token_id) != key); }
}
impl crate::behaviour::PoolTokenStorage for MemoryPorts {
fn get(&self, identity: &crate::token::TokenId) -> Option<crate::token::PoolTokenSnapshot> { crate::behaviour::PoolTokenStorage::get(&self.pool_token_storage, identity) }
fn put(&mut self, snapshot: crate::token::PoolTokenSnapshot) { crate::behaviour::PoolTokenStorage::put(&mut self.pool_token_storage, snapshot); }
fn delete(&mut self, identity: &crate::token::TokenId) { crate::behaviour::PoolTokenStorage::delete(&mut self.pool_token_storage, identity); }
}

/// Ephemeral storage of `conductor.tool.ToolCall`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryToolCallStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::tool::ToolCallSnapshot>>>);
impl crate::behaviour::ToolCallStorage for MemoryToolCallStorage {
fn get(&self, identity: &crate::tool::ToolCallId) -> Option<crate::tool::ToolCallSnapshot> { let key = memory_key_344(identity); self.0.borrow().iter().find(|row| memory_key_344(&row.data.tool_call_id) == key).cloned() }
fn put(&mut self, snapshot: crate::tool::ToolCallSnapshot) { self.delete(&snapshot.data.tool_call_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_344(&row.data.tool_call_id).cmp(&memory_key_344(&other.data.tool_call_id))); }
fn delete(&mut self, identity: &crate::tool::ToolCallId) { let key = memory_key_344(identity); self.0.borrow_mut().retain(|row| memory_key_344(&row.data.tool_call_id) != key); }
fn list(&self) -> Vec<crate::tool::ToolCallSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::ToolCallStorage for MemoryPorts {
fn get(&self, identity: &crate::tool::ToolCallId) -> Option<crate::tool::ToolCallSnapshot> { crate::behaviour::ToolCallStorage::get(&self.tool_call_storage, identity) }
fn put(&mut self, snapshot: crate::tool::ToolCallSnapshot) { crate::behaviour::ToolCallStorage::put(&mut self.tool_call_storage, snapshot); }
fn delete(&mut self, identity: &crate::tool::ToolCallId) { crate::behaviour::ToolCallStorage::delete(&mut self.tool_call_storage, identity); }
fn list(&self) -> Vec<crate::tool::ToolCallSnapshot> { crate::behaviour::ToolCallStorage::list(&self.tool_call_storage) }
}

/// Ports shared by generated components. Values disappear when the process exits.
#[derive(Clone, Default)]
pub struct MemoryPorts {
/// Storage of `conductor.agent.Agent`.
pub agent_storage: MemoryAgentStorage,
/// Storage of `conductor.agent.AgentKind`.
pub agent_kind_storage: MemoryAgentKindStorage,
/// Storage of `conductor.config.ConfigRevision`.
pub config_revision_storage: MemoryConfigRevisionStorage,
/// Storage of `conductor.decision.Decision`.
pub decision_storage: MemoryDecisionStorage,
/// Storage of `conductor.decision.DecisionRequest`.
pub decision_request_storage: MemoryDecisionRequestStorage,
/// Storage of `conductor.direction.Goal`.
pub goal_storage: MemoryGoalStorage,
/// Storage of `conductor.direction.GoalServing`.
pub goal_serving_storage: MemoryGoalServingStorage,
/// Storage of `conductor.direction.RepositoryMark`.
pub repository_mark_storage: MemoryRepositoryMarkStorage,
/// Storage of `conductor.dispatch.Controller`.
pub controller_storage: MemoryControllerStorage,
/// Storage of `conductor.dispatch.Dispatch`.
pub dispatch_storage: MemoryDispatchStorage,
/// Storage of `conductor.dispatch.GuardDecision`.
pub guard_decision_storage: MemoryGuardDecisionStorage,
/// Storage of `conductor.dispatch.Message`.
pub message_storage: MemoryMessageStorage,
/// Storage of `conductor.dispatch.ResourceRequest`.
pub resource_request_storage: MemoryResourceRequestStorage,
/// Storage of `conductor.host.HostGrant`.
pub host_grant_storage: MemoryHostGrantStorage,
/// Storage of `conductor.host.ReclaimRun`.
pub reclaim_run_storage: MemoryReclaimRunStorage,
/// Storage of `conductor.improvement.FrictionFinding`.
pub friction_finding_storage: MemoryFrictionFindingStorage,
/// Storage of `conductor.improvement.Metric`.
pub metric_storage: MemoryMetricStorage,
/// Storage of `conductor.improvement.MetricReading`.
pub metric_reading_storage: MemoryMetricReadingStorage,
/// Storage of `conductor.improvement.SettingsProposal`.
pub settings_proposal_storage: MemorySettingsProposalStorage,
/// Storage of `conductor.instance.Instance`.
pub instance_storage: MemoryInstanceStorage,
/// Storage of `conductor.instance.Reconciliation`.
pub reconciliation_storage: MemoryReconciliationStorage,
/// Storage of `conductor.observation.BlockerObservation`.
pub blocker_observation_storage: MemoryBlockerObservationStorage,
/// Storage of `conductor.observation.Board`.
pub board_storage: MemoryBoardStorage,
/// Storage of `conductor.observation.CollectorRunObservation`.
pub collector_run_observation_storage: MemoryCollectorRunObservationStorage,
/// Storage of `conductor.observation.DailySeries`.
pub daily_series_storage: MemoryDailySeriesStorage,
/// Storage of `conductor.observation.MemberFailureObservation`.
pub member_failure_observation_storage: MemoryMemberFailureObservationStorage,
/// Storage of `conductor.observation.MergedPullRequestObservation`.
pub merged_pull_request_observation_storage: MemoryMergedPullRequestObservationStorage,
/// Storage of `conductor.observation.PullRequestObservation`.
pub pull_request_observation_storage: MemoryPullRequestObservationStorage,
/// Storage of `conductor.observation.ReleaseObservation`.
pub release_observation_storage: MemoryReleaseObservationStorage,
/// Storage of `conductor.observation.RepositoryObservation`.
pub repository_observation_storage: MemoryRepositoryObservationStorage,
/// Storage of `conductor.observation.SecretSyncObservation`.
pub secret_sync_observation_storage: MemorySecretSyncObservationStorage,
/// Storage of `conductor.observation.SessionObservation`.
pub session_observation_storage: MemorySessionObservationStorage,
/// Storage of `conductor.observation.Snapshot`.
pub snapshot_storage: MemorySnapshotStorage,
/// Storage of `conductor.observation.SpecificationObservation`.
pub specification_observation_storage: MemorySpecificationObservationStorage,
/// Storage of `conductor.observation.UsageObservation`.
pub usage_observation_storage: MemoryUsageObservationStorage,
/// Storage of `conductor.observation.WorkflowRunObservation`.
pub workflow_run_observation_storage: MemoryWorkflowRunObservationStorage,
/// Storage of `conductor.operator.Operator`.
pub operator_storage: MemoryOperatorStorage,
/// Storage of `conductor.placement.Box`.
pub box_storage: MemoryBoxStorage,
/// Storage of `conductor.placement.BoxRefusal`.
pub box_refusal_storage: MemoryBoxRefusalStorage,
/// Storage of `conductor.placement.Placement`.
pub placement_storage: MemoryPlacementStorage,
/// Storage of `conductor.secret.Secret`.
pub secret_storage: MemorySecretStorage,
/// Storage of `conductor.session.Delivery`.
pub delivery_storage: MemoryDeliveryStorage,
/// Storage of `conductor.session.Lease`.
pub lease_storage: MemoryLeaseStorage,
/// Storage of `conductor.session.RunTick`.
pub run_tick_storage: MemoryRunTickStorage,
/// Storage of `conductor.store.Build`.
pub build_storage: MemoryBuildStorage,
/// Storage of `conductor.store.Migration`.
pub migration_storage: MemoryMigrationStorage,
/// Storage of `conductor.store.Store`.
pub store_storage: MemoryStoreStorage,
/// Storage of `conductor.token.PoolToken`.
pub pool_token_storage: MemoryPoolTokenStorage,
/// Storage of `conductor.tool.ToolCall`.
pub tool_call_storage: MemoryToolCallStorage,
}

impl crate::behaviour::TryContext for MemoryPorts {
fn try_generate_conductor_config_revision_id(&mut self) -> Result<crate::config::RevisionId, crate::obligation::UnmetObligation> { Ok(crate::config::RevisionId(crate::primitives::Uuid(uuid::Uuid::new_v4().to_string()))) }
fn try_generate_conductor_direction_serving_id(&mut self) -> Result<crate::direction::ServingId, crate::obligation::UnmetObligation> { Ok(crate::direction::ServingId(crate::primitives::Uuid(uuid::Uuid::new_v4().to_string()))) }
fn try_generate_conductor_dispatch_guard_decision_id(&mut self) -> Result<crate::dispatch::GuardDecisionId, crate::obligation::UnmetObligation> { Ok(crate::dispatch::GuardDecisionId(crate::primitives::Uuid(uuid::Uuid::new_v4().to_string()))) }
fn try_generate_conductor_dispatch_message_id(&mut self) -> Result<crate::dispatch::MessageId, crate::obligation::UnmetObligation> { Ok(crate::dispatch::MessageId(crate::primitives::Uuid(uuid::Uuid::new_v4().to_string()))) }
fn try_generate_conductor_dispatch_resource_request_id(&mut self) -> Result<crate::dispatch::ResourceRequestId, crate::obligation::UnmetObligation> { Ok(crate::dispatch::ResourceRequestId(crate::primitives::Uuid(uuid::Uuid::new_v4().to_string()))) }
fn try_generate_conductor_host_grant_id(&mut self) -> Result<crate::host::GrantId, crate::obligation::UnmetObligation> { Err(crate::behaviour::unmet_context("assigned value: conductor.host.GrantId")) }
fn try_generate_conductor_host_reclaim_run_id(&mut self) -> Result<crate::host::ReclaimRunId, crate::obligation::UnmetObligation> { Err(crate::behaviour::unmet_context("assigned value: conductor.host.ReclaimRunId")) }
fn try_generate_conductor_instance_reconciliation_id(&mut self) -> Result<crate::instance::ReconciliationId, crate::obligation::UnmetObligation> { Ok(crate::instance::ReconciliationId(crate::primitives::Uuid(uuid::Uuid::new_v4().to_string()))) }
fn try_generate_conductor_observation_board_id(&mut self) -> Result<crate::observation::BoardId, crate::obligation::UnmetObligation> { Ok(crate::observation::BoardId(crate::primitives::Uuid(uuid::Uuid::new_v4().to_string()))) }
fn try_generate_conductor_observation_observation_id(&mut self) -> Result<crate::observation::ObservationId, crate::obligation::UnmetObligation> { Ok(crate::observation::ObservationId(crate::primitives::Uuid(uuid::Uuid::new_v4().to_string()))) }
fn try_generate_conductor_observation_snapshot_id(&mut self) -> Result<crate::observation::SnapshotId, crate::obligation::UnmetObligation> { Ok(crate::observation::SnapshotId(crate::primitives::Uuid(uuid::Uuid::new_v4().to_string()))) }
fn try_generate_conductor_placement_box_id(&mut self) -> Result<crate::placement::BoxId, crate::obligation::UnmetObligation> { Ok(crate::placement::BoxId(crate::primitives::Uuid(uuid::Uuid::new_v4().to_string()))) }
fn try_generate_conductor_placement_refusal_reason(&mut self) -> Result<crate::placement::RefusalReason, crate::obligation::UnmetObligation> { Err(crate::behaviour::unmet_context("assigned value: conductor.placement.RefusalReason")) }
fn try_generate_conductor_session_delivery_id(&mut self) -> Result<crate::session::DeliveryId, crate::obligation::UnmetObligation> { Ok(crate::session::DeliveryId(crate::primitives::Uuid(uuid::Uuid::new_v4().to_string()))) }
fn try_generate_conductor_session_tick_id(&mut self) -> Result<crate::session::TickId, crate::obligation::UnmetObligation> { Ok(crate::session::TickId(crate::primitives::Uuid(uuid::Uuid::new_v4().to_string()))) }
fn try_generate_conductor_token_token_id(&mut self) -> Result<crate::token::TokenId, crate::obligation::UnmetObligation> { Ok(crate::token::TokenId(crate::primitives::Uuid(uuid::Uuid::new_v4().to_string()))) }
fn try_generate_conductor_tool_tool_call_id(&mut self) -> Result<crate::tool::ToolCallId, crate::obligation::UnmetObligation> { Ok(crate::tool::ToolCallId(crate::primitives::Uuid(uuid::Uuid::new_v4().to_string()))) }
fn try_generate_string(&mut self) -> Result<String, crate::obligation::UnmetObligation> { Err(crate::behaviour::unmet_context("assigned value: String")) }
fn try_external(&mut self, _command: crate::behaviour::ExternalCommand<'_>, _outcome: &'static str) -> Result<bool, crate::obligation::UnmetObligation> { Err(crate::behaviour::unmet_context("external branch answer")) }
}

fn memory_key_2(value: &crate::agent::AgentKindName) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).clone()) }

fn memory_key_3(value: &crate::agent::AgentName) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).clone()) }

fn memory_key_49(value: &crate::config::InstanceName) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).clone()) }

fn memory_key_74(value: &crate::config::RevisionId) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).0.clone()) }

fn memory_key_108(value: &crate::decision::DecisionId) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).clone()) }

fn memory_key_114(value: &crate::decision::RequestId) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).clone()) }

fn memory_key_121(value: &crate::direction::GoalId) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).clone()) }

fn memory_key_124(value: &crate::direction::MarkedRepository) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).clone()) }

fn memory_key_143(value: &crate::direction::ServingId) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).0.clone()) }

fn memory_key_147(value: &crate::dispatch::ControllerId) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).0.clone()) }

fn memory_key_149(value: &crate::dispatch::DispatchId) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).clone()) }

fn memory_key_153(value: &crate::dispatch::GuardDecisionId) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).0.clone()) }

fn memory_key_158(value: &crate::dispatch::MessageId) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).0.clone()) }

fn memory_key_164(value: &crate::dispatch::ResourceRequestId) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).0.clone()) }

fn memory_key_175(value: &crate::host::GrantId) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).clone()) }

fn memory_key_184(value: &crate::host::ReclaimRunId) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).clone()) }

fn memory_key_189(value: &crate::improvement::FindingId) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).clone()) }

fn memory_key_192(value: &crate::improvement::MetricId) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).clone()) }

fn memory_key_194(value: &crate::improvement::MetricReadingId) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).clone()) }

fn memory_key_195(value: &crate::improvement::ProposalId) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).clone()) }

fn memory_key_205(value: &crate::instance::ReconciliationId) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).0.clone()) }

fn memory_key_211(value: &crate::observation::BoardId) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).0.clone()) }

fn memory_key_217(value: &crate::observation::DailySeriesId) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).clone()) }

fn memory_key_229(value: &crate::observation::ObservationId) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).0.clone()) }

fn memory_key_243(value: &crate::observation::SnapshotId) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).0.clone()) }

fn memory_key_263(value: &crate::operator::OperatorKey) -> MemoryKey { let _ = value; MemoryKey::List(vec![memory_key_264(&value.operator), memory_key_49(&value.instance)]) }

fn memory_key_264(value: &crate::operator::OperatorName) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).clone()) }

fn memory_key_267(value: &crate::placement::BoxId) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).0.clone()) }

fn memory_key_275(value: &crate::placement::PlacementName) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).clone()) }

fn memory_key_280(value: &crate::secret::SecretName) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).clone()) }

fn memory_key_290(value: &crate::session::DeliveryId) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).0.clone()) }

fn memory_key_308(value: &crate::session::TickId) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).0.clone()) }

fn memory_key_311(value: &crate::store::BinaryVersion) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).clone()) }

fn memory_key_315(value: &crate::store::StoreUrl) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).clone()) }

fn memory_key_321(value: &crate::token::TokenId) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).0.clone()) }

fn memory_key_344(value: &crate::tool::ToolCallId) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).0.clone()) }

// A common structural order; individual specifications use only a subset of the variants.
#[allow(dead_code)]
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum MemoryKey {
    Null,
    Boolean(bool),
    Integer(i64),
    Number(String),
    Text(String),
    Bytes(Vec<u8>),
    List(Vec<MemoryKey>),
    Map(Vec<(MemoryKey, MemoryKey)>),
    Optional(Option<Box<MemoryKey>>),
}

impl crate::agent::obligations::AttachAgentBehavior for MemoryPorts {
fn attach_agent(&mut self, input: crate::agent::AttachAgent) -> Result<crate::agent::AttachAgentOutcome, crate::obligation::UnmetObligation> { crate::agent::obligations::AttachAgentBehavior::attach_agent(&mut crate::agent::obligations::Unimplemented, input) }
}
impl crate::agent::obligations::DeliverToAgentBehavior for MemoryPorts {
fn deliver_to_agent(&mut self, input: crate::agent::DeliverToAgent) -> Result<crate::agent::DeliverToAgentOutcome, crate::obligation::UnmetObligation> { crate::agent::obligations::DeliverToAgentBehavior::deliver_to_agent(&mut crate::agent::obligations::Unimplemented, input) }
}
impl crate::agent::obligations::StartAgentBehavior for MemoryPorts {
fn start_agent(&mut self, input: crate::agent::StartAgent) -> Result<crate::agent::StartAgentOutcome, crate::obligation::UnmetObligation> { crate::agent::obligations::StartAgentBehavior::start_agent(&mut crate::agent::obligations::Unimplemented, input) }
}
impl crate::decision::obligations::RecordOperatorDecisionForRequestBehavior for MemoryPorts {
fn record_operator_decision_for_request(&mut self, input: crate::decision::RecordOperatorDecisionForRequest) -> Result<crate::decision::RecordOperatorDecisionForRequestOutcome, crate::obligation::UnmetObligation> { crate::decision::obligations::RecordOperatorDecisionForRequestBehavior::record_operator_decision_for_request(&mut crate::decision::obligations::Unimplemented, input) }
}
impl crate::dispatch::obligations::StartControllerBehavior for MemoryPorts {
fn start_controller(&mut self, input: crate::dispatch::StartController) -> Result<crate::dispatch::StartControllerOutcome, crate::obligation::UnmetObligation> { crate::dispatch::obligations::StartControllerBehavior::start_controller(&mut crate::dispatch::obligations::Unimplemented, input) }
}
impl crate::improvement::obligations::SetMetricTargetBehavior for MemoryPorts {
fn set_metric_target(&mut self, input: crate::improvement::SetMetricTarget) -> Result<crate::improvement::SetMetricTargetOutcome, crate::obligation::UnmetObligation> { crate::improvement::obligations::SetMetricTargetBehavior::set_metric_target(&mut crate::improvement::obligations::Unimplemented, input) }
}
impl crate::instance::obligations::StartInstanceBehavior for MemoryPorts {
fn start_instance(&mut self, input: crate::instance::StartInstance) -> Result<crate::instance::StartInstanceOutcome, crate::obligation::UnmetObligation> { crate::instance::obligations::StartInstanceBehavior::start_instance(&mut crate::instance::obligations::Unimplemented, input) }
}
impl crate::secret::obligations::PutSecretBehavior for MemoryPorts {
fn put_secret(&mut self, input: crate::secret::PutSecret) -> Result<crate::secret::PutSecretOutcome, crate::obligation::UnmetObligation> { crate::secret::obligations::PutSecretBehavior::put_secret(&mut crate::secret::obligations::Unimplemented, input) }
}
impl crate::secret::obligations::ResolveSecretsBehavior for MemoryPorts {
fn resolve_secrets(&mut self, input: crate::secret::ResolveSecrets) -> Result<crate::secret::ResolveSecretsOutcome, crate::obligation::UnmetObligation> { crate::secret::obligations::ResolveSecretsBehavior::resolve_secrets(&mut crate::secret::obligations::Unimplemented, input) }
}
impl crate::session::obligations::ClaimLeaseBehavior for MemoryPorts {
fn claim_lease(&mut self, input: crate::session::ClaimLease) -> Result<crate::session::ClaimLeaseOutcome, crate::obligation::UnmetObligation> { crate::session::obligations::ClaimLeaseBehavior::claim_lease(&mut crate::session::obligations::Unimplemented, input) }
}
impl crate::store::obligations::MigrateStoreBehavior for MemoryPorts {
fn migrate_store(&mut self, input: crate::store::MigrateStore) -> Result<crate::store::MigrateStoreOutcome, crate::obligation::UnmetObligation> { crate::store::obligations::MigrateStoreBehavior::migrate_store(&mut crate::store::obligations::Unimplemented, input) }
}
impl crate::config::obligations::EffectiveSettingsQuery for MemoryPorts {
fn effective_settings(&self, instance: crate::config::InstanceName) -> Result<Vec<crate::config::EffectiveSettings>, crate::obligation::UnmetObligation> { crate::config::obligations::EffectiveSettingsQuery::effective_settings(&crate::config::obligations::Unimplemented, instance) }
}
impl crate::instance::obligations::InstanceByNameQuery for MemoryPorts {
fn instance_by_name(&self, name: crate::config::InstanceName) -> Result<Vec<crate::instance::InstanceByName>, crate::obligation::UnmetObligation> { crate::instance::obligations::InstanceByNameQuery::instance_by_name(&crate::instance::obligations::Unimplemented, name) }
}
impl crate::token::obligations::TokensQuery for MemoryPorts {
fn tokens(&self, instance: crate::config::InstanceName) -> Result<Vec<crate::token::Tokens>, crate::obligation::UnmetObligation> { crate::token::obligations::TokensQuery::tokens(&crate::token::obligations::Unimplemented, instance) }
}
