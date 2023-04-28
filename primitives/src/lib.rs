#![cfg_attr(not(feature = "std"), no_std)]

mod unchecked_extrinsic;

use frame_support::{
	dispatch::{DispatchInfo, GetDispatchInfo},
	traits::ExtrinsicCall,
};
use sp_runtime::traits::SignedExtension;

pub use unchecked_extrinsic::UncheckedExtrinsic;

impl<Address, Call, Signature, Extra> ExtrinsicCall
	for UncheckedExtrinsic<Address, Call, Signature, Extra>
where
	Extra: sp_runtime::traits::SignedExtension,
{
	fn call(&self) -> &Self::Call {
		&self.function
	}
}

/// Implementation for unchecked extrinsic.
impl<Address, Call, Signature, Extra> GetDispatchInfo
	for UncheckedExtrinsic<Address, Call, Signature, Extra>
where
	Call: GetDispatchInfo,
	Extra: SignedExtension,
{
	fn get_dispatch_info(&self) -> DispatchInfo {
		self.function.get_dispatch_info()
	}
}
