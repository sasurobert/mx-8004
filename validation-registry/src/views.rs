multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::structs::JobData;

#[multiversx_sc::module]
pub trait ViewsModule:
    common::cross_contract::CrossContractModule + crate::storage::ExternalStorageModule
{
    #[view(isJobVerified)]
    fn is_job_verified(&self, job_id: ManagedBuffer) -> bool {
        let job_mapper = self.job_data(&job_id);
        !job_mapper.is_empty() && job_mapper.get().status == crate::structs::JobStatus::Verified
    }

    #[view(getJobData)]
    fn get_job_data(&self, job_id: ManagedBuffer) -> OptionalValue<JobData<Self::Api>> {
        let job_mapper = self.job_data(&job_id);
        if job_mapper.is_empty() {
            OptionalValue::None
        } else {
            OptionalValue::Some(job_mapper.get())
        }
    }
}
