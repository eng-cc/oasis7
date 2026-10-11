use super::*;
impl Runtime {
    pub fn create_session(
        &mut self,
        project: &Project,
        config: SessionConfig,
    ) -> Result<RuntimeSnapshot> {
        let scope = self.gate(project, true)?;
        if config.purpose.trim().is_empty()
            || config.scope.trim().is_empty()
            || config.purpose.len() > 4096
            || config.scope.len() > 4096
            || config.model.as_ref().is_some_and(|m| m.len() > 128)
            || config.basis_revision != crate::index::project(Path::new(&project.path))?.head
        {
            return Err(Error::Invalid(
                "explicit bounded purpose/scope and current basis revision required".into(),
            ));
        }
        if self.connections[&scope].snapshot().sessions.len() >= 128 {
            return Err(Error::Invalid("session window exhausted".into()));
        }
        if let Some(value)=self.perform(&scope,"thread/start",None,None,json!({"cwd":project.path,"model":config.model,"approvalPolicy":"on-request","sandbox":"workspace-write","ephemeral":false}),false)?{
        let thread=value["thread"]["id"].as_str().filter(|id|!id.is_empty()).ok_or_else(||Error::Invalid("thread/start response lacks identity".into()))?.to_string();
        let connection=&self.connections[&scope];let mut shared=connection.shared.lock().unwrap();let instance=shared.snapshot.instance_id.clone().unwrap();
        shared.snapshot.sessions.push(RuntimeSession{purpose:config.purpose,scope:config.scope,model:value["model"].as_str().map(str::to_owned),basis_revision:config.basis_revision,thread_id:thread,instance_id:instance,turn_id:None,state:"idle".into(),owned:true,background_state:"clear".into(),restorable_history:false});}
        self.snapshot(project)
    }
    pub fn continue_session(
        &mut self,
        project: &Project,
        thread_id: &str,
    ) -> Result<RuntimeSnapshot> {
        let scope = self.gate(project, true)?;
        let snap = self.connections[&scope].snapshot();
        let session = snap
            .sessions
            .iter()
            .find(|s| s.owned && s.thread_id == thread_id)
            .ok_or_else(|| {
                Error::Invalid("history access does not grant external control".into())
            })?;
        if !session.restorable_history
            && (!["idle", "completed"].contains(&session.state.as_str())
                || session.background_state != "clear")
        {
            return Err(Error::Invalid(
                "active or unknown session cannot be resumed as new work".into(),
            ));
        }
        if self.store.operations(&scope)?.iter().any(|o| {
            o.thread_id.as_deref() == Some(thread_id)
                && ["pending", "awaiting", "unknown"].contains(&o.state.as_str())
        }) || snap.requests.iter().any(|r| {
            r.thread_id == thread_id
                && ["pending", "awaiting", "unknown"].contains(&r.state.as_str())
        }) {
            return Err(Error::Invalid(
                "original unresolved target cannot be resumed".into(),
            ));
        }
        if let Some(value) = self.perform(
            &scope,
            "thread/resume",
            Some(thread_id),
            None,
            json!({"threadId":thread_id,"cwd":project.path}),
            false,
        )? {
            if value["thread"]["id"].as_str() != Some(thread_id) {
                return Err(Error::Invalid(
                    "resume returned another thread identity".into(),
                ));
            }
            let mut shared = self.connections[&scope].shared.lock().unwrap();
            let instance = shared.snapshot.instance_id.clone().unwrap();
            if let Some(s) = shared
                .snapshot
                .sessions
                .iter_mut()
                .find(|s| s.thread_id == thread_id)
            {
                s.instance_id = instance;
                s.restorable_history = false;
                s.state = match value["thread"]["status"]["type"].as_str() {
                    Some("idle") => "idle",
                    Some("active") => "active",
                    _ => "unknown",
                }
                .into();
                s.turn_id = value["thread"]["turns"]
                    .as_array()
                    .and_then(|turns| turns.iter().rev().find(|t| t["status"] == "inProgress"))
                    .and_then(|turn| turn["id"].as_str())
                    .map(str::to_owned);
                s.background_state = "unknown".into();
            }
        }
        self.snapshot(project)
    }
    pub fn start_turn(
        &mut self,
        project: &Project,
        thread_id: &str,
        input: &str,
    ) -> Result<RuntimeSnapshot> {
        let scope = self.gate(project, true)?;
        self.observe(&scope);
        Self::input(input)?;
        let snap = self.connections[&scope].snapshot();
        if !snap.sessions.iter().any(|s| {
            s.owned
                && s.thread_id == thread_id
                && Some(&s.instance_id) == snap.instance_id.as_ref()
                && ["idle", "completed"].contains(&s.state.as_str())
                && s.background_state == "clear"
        }) {
            return Err(Error::Invalid(
                "start requires owned idle thread and confirmed background scope".into(),
            ));
        }
        if let Some(value) = self.perform(
            &scope,
            "turn/start",
            Some(thread_id),
            None,
            json!({"threadId":thread_id,"input":[{"type":"text","text":input}]}),
            false,
        )? {
            let id = value["turn"]["id"]
                .as_str()
                .ok_or_else(|| Error::Invalid("turn/start response lacks turn identity".into()))?
                .to_string();
            let mut shared = self.connections[&scope].shared.lock().unwrap();
            let s = shared
                .snapshot
                .sessions
                .iter_mut()
                .find(|s| s.thread_id == thread_id)
                .unwrap();
            if s.turn_id.as_deref() != Some(&id) {
                s.turn_id = Some(id);
                s.state = match value["turn"]["status"].as_str() {
                    Some("failed") => "failed",
                    Some("interrupted") => "interrupted",
                    Some("completed") => "completed",
                    _ => "active",
                }
                .into();
                s.background_state = "unknown".into();
            }
        }
        self.snapshot(project)
    }
    fn input(input: &str) -> Result<()> {
        if input.trim().is_empty() || input.len() > 32768 {
            return Err(Error::Invalid(
                "input must be nonempty and at most 32 KiB".into(),
            ));
        }
        Ok(())
    }
    pub fn steer_turn(
        &mut self,
        project: &Project,
        target: RuntimeTarget,
        input: &str,
    ) -> Result<RuntimeSnapshot> {
        let scope = self.gate(project, true)?;
        Self::input(input)?;
        self.target(&scope, &target)?;
        self.perform(&scope,"turn/steer",Some(&target.thread_id),Some(&target.turn_id),json!({"threadId":target.thread_id,"expectedTurnId":target.turn_id,"input":[{"type":"text","text":input}]}),false)?;
        self.snapshot(project)
    }
    pub fn interrupt_turn(
        &mut self,
        project: &Project,
        target: RuntimeTarget,
    ) -> Result<RuntimeSnapshot> {
        let scope = self.gate(project, false)?;
        self.target(&scope, &target)?;
        self.perform(
            &scope,
            "turn/interrupt",
            Some(&target.thread_id),
            Some(&target.turn_id),
            json!({"threadId":target.thread_id,"turnId":target.turn_id}),
            true,
        )?;
        self.snapshot(project)
    }
    pub fn answer_request(
        &mut self,
        project: &Project,
        reply: RequestAnswer,
    ) -> Result<RuntimeSnapshot> {
        let scope = self.gate(project, false)?;
        self.target(&scope, &reply.target)?;
        let connection = &self.connections[&scope];
        let snap = connection.snapshot();
        let request = snap
            .requests
            .iter()
            .find(|r| {
                r.request_id == reply.request_id
                    && r.instance_id == reply.target.instance_id
                    && r.thread_id == reply.target.thread_id
                    && r.turn_id == reply.target.turn_id
            })
            .ok_or_else(|| Error::Invalid("request is not present in this instance".into()))?;
        let result = protocol::answer(request, &reply)?;
        let raw_id: Value = serde_json::from_str(&request.request_id)?;
        let mut op = RuntimeOperation {
            id: identity(),
            instance_id: reply.target.instance_id,
            thread_id: Some(reply.target.thread_id.clone()),
            turn_id: Some(reply.target.turn_id),
            method: format!("answerRequest:{}", reply.request_id),
            state: "pending".into(),
            fingerprint: crate::index::digest(&serde_json::to_vec(&result)?),
            created_at: crate::now(),
            error: None,
        };
        let duplicate:bool=self.store.db.query_row("SELECT EXISTS(SELECT 1 FROM cc_runtime_operations WHERE scope=?1 AND state!='failed' AND json_extract(value,'$.method')=?2 AND json_extract(value,'$.instanceId')=?3 AND json_extract(value,'$.threadId')=?4 AND json_extract(value,'$.turnId')=?5)",rusqlite::params![scope,op.method,op.instance_id,op.thread_id,op.turn_id],|row|row.get(0))?;
        if duplicate {
            return Err(Error::Invalid(
                "original request already has a durable reply receipt; no automatic resend".into(),
            ));
        }
        self.store
            .insert(&scope, &reply.target.thread_id, &op, true)?;
        match connection.send(json!({"id":raw_id,"result":result})) {
            Ok(()) => {
                op.state = "awaiting".into();
            }
            Err(_) => {
                op.state = "unknown".into();
                op.error = Some("reply delivery unconfirmed".into());
            }
        }
        self.store.settle(&op)?;
        {
            let mut shared = connection.shared.lock().unwrap();
            if let Some(r) = shared
                .snapshot
                .requests
                .iter_mut()
                .find(|r| r.request_id == reply.request_id)
                && r.state != "resolved"
            {
                r.state = op.state.clone();
            }
        }
        self.snapshot(project)
    }
    pub fn read_thread(&mut self, project: &Project, thread_id: &str) -> Result<RuntimeSnapshot> {
        let scope = self.gate(project, false)?;
        let value = self.connections[&scope]
            .request(
                &identity(),
                "thread/read",
                json!({"threadId":thread_id,"includeTurns":false}),
            )
            .map_err(Error::Invalid)?;
        if value["thread"]["id"].as_str() != Some(thread_id) {
            return Err(Error::Invalid("read returned another thread".into()));
        }
        self.snapshot(project)
    }
}

impl Runtime {
    pub fn reconcile_operation(
        &mut self,
        project: &Project,
        operation_id: &str,
    ) -> Result<RuntimeSnapshot> {
        let scope = self.gate(project, false)?;
        let mut op = self
            .store
            .operations(&scope)?
            .into_iter()
            .find(|o| o.id == operation_id)
            .ok_or_else(|| Error::Invalid("operation not in selected project".into()))?;
        let connection = &self.connections[&scope];
        let snapshot = connection.snapshot();
        if snapshot.instance_id.as_deref() != Some(&op.instance_id)
            || !["pending", "awaiting", "unknown"].contains(&op.state.as_str())
        {
            return Err(Error::Invalid(
                "only unresolved original-instance operations can be reconciled".into(),
            ));
        }
        let thread = op.thread_id.as_deref().ok_or_else(|| {
            Error::Invalid("unknown new-session identity cannot be inferred or replayed".into())
        })?;
        let value = connection
            .request(
                &identity(),
                "thread/read",
                json!({"threadId":thread,"includeTurns":false}),
            )
            .map_err(Error::Invalid)?;
        if value["thread"]["id"].as_str() != Some(thread)
            || value["thread"]["status"]["type"].as_str() != Some("idle")
        {
            return Err(Error::Invalid(
                "source has not confirmed the original thread idle".into(),
            ));
        }
        let processes = lifecycle::capture_owned(connection.child.id())?;
        if processes
            .iter()
            .any(|pid| !connection.baseline.iter().any(|old| old.same_birth(pid)))
        {
            return Err(Error::Invalid(
                "owned background processes still active or unknown".into(),
            ));
        }
        if snapshot.requests.iter().any(|r| {
            r.instance_id == op.instance_id
                && r.thread_id == thread
                && ["pending", "awaiting", "unknown"].contains(&r.state.as_str())
        }) {
            return Err(Error::Invalid(
                "original runtime still has unresolved user requests".into(),
            ));
        }
        op.state = "sourceResolved".into();
        op.error=Some("Read-only source/process audit confirms no active work; original input effect remains unverified and is never replayed".into());
        self.store.settle(&op)?;
        {
            let mut shared = connection.shared.lock().unwrap();
            if let Some(session) = shared
                .snapshot
                .sessions
                .iter_mut()
                .find(|s| s.thread_id == thread && s.instance_id == op.instance_id)
            {
                session.state = "idle".into();
                session.background_state = "clear".into();
            }
            if shared.snapshot.state == "degraded" {
                shared.snapshot.state = "ready".into();
                shared.snapshot.error = None;
            }
        }
        self.snapshot(project)
    }
}
