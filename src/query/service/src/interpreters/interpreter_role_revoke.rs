// Copyright 2021 Datafuse Labs.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::sync::Arc;

use common_exception::Result;
use common_legacy_planners::RevokeRolePlan;
use common_meta_types::PrincipalIdentity;
use common_streams::DataBlockStream;
use common_streams::SendableDataBlockStream;

use crate::interpreters::Interpreter;
use crate::sessions::QueryContext;
use crate::sessions::TableContext;

#[derive(Debug)]
pub struct RevokeRoleInterpreter {
    ctx: Arc<QueryContext>,
    plan: RevokeRolePlan,
}

impl RevokeRoleInterpreter {
    pub fn try_create(ctx: Arc<QueryContext>, plan: RevokeRolePlan) -> Result<Self> {
        Ok(RevokeRoleInterpreter { ctx, plan })
    }
}

#[async_trait::async_trait]
impl Interpreter for RevokeRoleInterpreter {
    fn name(&self) -> &str {
        "RevokeRoleInterpreter"
    }

    #[tracing::instrument(level = "debug", skip(self), fields(ctx.id = self.ctx.get_id().as_str()))]
    async fn execute(&self) -> Result<SendableDataBlockStream> {
        let plan = self.plan.clone();
        let tenant = self.ctx.get_tenant();
        let user_mgr = self.ctx.get_user_manager();
        match plan.principal {
            PrincipalIdentity::User(user) => {
                user_mgr
                    .revoke_role_from_user(&tenant, user, plan.role)
                    .await?;
            }
            PrincipalIdentity::Role(role) => {
                user_mgr
                    .revoke_role_from_role(&tenant, role.clone(), plan.role)
                    .await?;
            }
        }

        Ok(Box::pin(DataBlockStream::create(
            self.plan.schema(),
            None,
            vec![],
        )))
    }
}
