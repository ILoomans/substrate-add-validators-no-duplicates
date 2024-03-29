// //! # Validator Set Pallet
// //!
// //! The Validator Set Pallet allows addition and removal of authorities/validators
// //! via extrinsics (transaction calls), in Substrate-based PoA networks.
// //! It also integrates with the im-online pallet to automatically remove offline validators.
// //!
// //! The pallet uses the Session pallet and implements related traits for session
// //! management. Currently it uses periodic session rotation provided by the
// //! session pallet to automatically rotate sessions. For this reason,
// //! the validator addition and removal becomes effective only after 2 sessions (queuing + applying).

// #![cfg_attr(not(feature = "std"), no_std)]

// mod mock;
// mod tests;

// use frame_support::{
// 	ensure,
// 	traits::{EstimateNextSessionRotation, Get, ValidatorSet, ValidatorSetWithIdentification},
// };
// use log;
// pub use pallet::*;
// use sp_runtime::{
// 	traits::{Convert, Zero},
// 	DispatchError,
// };
// use sp_staking::offence::{Offence, OffenceError, ReportOffence};
// use sp_std::{collections::btree_set::BTreeSet, prelude::*};

// pub const LOG_TARGET: &'static str = "runtime::validator-set";

// #[frame_support::pallet]
// pub mod pallet {
// 	use super::*;
// 	use frame_support::pallet_prelude::*;
// 	use frame_system::pallet_prelude::*;

// 	/// Configure the pallet by specifying the parameters and types on which it depends.
// 	#[pallet::config]
// 	pub trait Config: frame_system::Config + pallet_session::Config {
// 		/// The Event type.
// 		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

// 		/// Origin for adding or removing a validator.
// 		type AddRemoveOrigin: EnsureOrigin<Self::Origin>;

// 		/// Minimum number of validators to leave in the validator set during auto removal.
// 		type MinAuthorities: Get<u32>;
// 	}

// 	#[pallet::pallet]
// 	#[pallet::generate_store(pub(super) trait Store)]
// 	pub struct Pallet<T>(_);

// 	#[pallet::storage]
// 	#[pallet::getter(fn validators)]
// 	pub type Validators<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

// 	#[pallet::storage]
// 	#[pallet::getter(fn validators_to_remove)]
// 	pub type OfflineValidators<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

// 	#[pallet::event]
// 	#[pallet::generate_deposit(pub(super) fn deposit_event)]
// 	pub enum Event<T: Config> {
// 		/// New validator addition initiated. Effective in ~2 sessions.
// 		ValidatorAdditionInitiated(T::AccountId),

// 		/// Validator removal initiated. Effective in ~2 sessions.
// 		ValidatorRemovalInitiated(T::AccountId),
// 	}

// 	// Errors inform users that something went wrong.
// 	#[pallet::error]
// 	pub enum Error<T> {
// 		/// Target (post-removal) validator count is below the minimum.
// 		TooLowValidatorCount,
// 		/// Validator is already in the validator set.
// 		Duplicate,
// 	}

// 	#[pallet::hooks]
// 	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

// 	#[pallet::genesis_config]
// 	pub struct GenesisConfig<T: Config> {
// 		pub initial_validators: Vec<T::AccountId>,
// 	}

// 	#[cfg(feature = "std")]
// 	impl<T: Config> Default for GenesisConfig<T> {
// 		fn default() -> Self {
// 			Self { initial_validators: Default::default() }
// 		}
// 	}

// 	#[pallet::genesis_build]
// 	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
// 		fn build(&self) {
// 			Pallet::<T>::initialize_validators(&self.initial_validators);
// 		}
// 	}

// 	#[pallet::call]
// 	impl<T: Config> Pallet<T> {
// 		/// Add a new validator.
// 		///
// 		/// New validator's session keys should be set in session module before calling this.
// 		/// Use `author::set_keys()` RPC to set keys.
// 		///
// 		/// The origin can be configured using the `AddRemoveOrigin` type in the host runtime.
// 		/// Can also be set to sudo/root.
// 		#[pallet::weight(0)]
// 		pub fn add_validator(origin: OriginFor<T>, validator_id: T::AccountId) -> DispatchResult {
// 			T::AddRemoveOrigin::ensure_origin(origin)?;

// 			Self::do_add_validator(validator_id)?;

// 			Ok(())
// 		}

// 		/// Remove a validator.
// 		///
// 		/// The origin can be configured using the `AddRemoveOrigin` type in the host runtime.
// 		/// Can also be set to sudo/root.
// 		#[pallet::weight(0)]
// 		pub fn remove_validator(
// 			origin: OriginFor<T>,
// 			validator_id: T::AccountId,
// 		) -> DispatchResult {
// 			T::AddRemoveOrigin::ensure_origin(origin)?;

// 			Self::do_remove_validator(validator_id)?;

// 			Ok(())
// 		}
// 	}
// }

// impl<T: Config> Pallet<T> {
// 	fn initialize_validators(validators: &[T::AccountId]) {
// 		assert!(validators.len() > 1, "At least 2 validators should be initialized");
// 		assert!(<Validators<T>>::get().is_empty(), "Validators are already initialized!");
// 		<Validators<T>>::put(validators);
// 	}

// 	fn do_add_validator(validator_id: T::AccountId) -> Result<T::AccountId, DispatchError> {
// 		let validator_set: BTreeSet<_> = <Validators<T>>::get().into_iter().collect();
// 		ensure!(!validator_set.contains(&validator_id), Error::<T>::Duplicate);
// 		<Validators<T>>::mutate(|v| v.push(validator_id.clone()));

// 		Self::deposit_event(Event::ValidatorAdditionInitiated(validator_id.clone()));
// 		log::debug!(target: LOG_TARGET, "Validator addition initiated.");

// 		Ok(validator_id)
// 	}

// 	fn do_remove_validator(validator_id: T::AccountId) -> Result<T::AccountId, DispatchError> {
// 		let mut validators = <Validators<T>>::get();

// 		// Ensuring that the post removal, target validator count doesn't go below the minimum.
// 		ensure!(
// 			validators.len().saturating_sub(1) as u32 >= T::MinAuthorities::get(),
// 			Error::<T>::TooLowValidatorCount
// 		);

// 		validators.retain(|v| *v != validator_id);

// 		<Validators<T>>::put(validators);

// 		Self::deposit_event(Event::ValidatorRemovalInitiated(validator_id.clone()));
// 		log::debug!(target: LOG_TARGET, "Validator removal initiated.");

// 		Ok(validator_id)
// 	}

// 	// Adds offline validators to a local cache for removal at new session.
// 	fn mark_for_removal(validator_id: T::AccountId) {
// 		<OfflineValidators<T>>::mutate(|v| v.push(validator_id));
// 		log::debug!(target: LOG_TARGET, "Offline validator marked for auto removal.");
// 	}

// 	// Removes offline validators from the validator set and clears the offline cache.
// 	fn remove_offline_validators() {
// 		let validators_to_remove: BTreeSet<_> = <OfflineValidators<T>>::get().into_iter().collect();

// 		// Delete from active validator set.
// 		<Validators<T>>::mutate(|vs| vs.retain(|v| !validators_to_remove.contains(v)));
// 		log::debug!(
// 			target: LOG_TARGET,
// 			"Initiated removal of {:?} offline validators.",
// 			validators_to_remove.len()
// 		);

// 		// Clear the offline validator list to avoid repeated deletion.
// 		<OfflineValidators<T>>::put(Vec::<T::AccountId>::new());
// 	}
// }

// // Provides the new set of validators to the session module when session is being rotated.
// impl<T: Config> pallet_session::SessionManager<T::AccountId> for Pallet<T> {
// 	// Plan a new session and provide new validator set.
// 	fn new_session(_new_index: u32) -> Option<Vec<T::AccountId>> {
// 		Self::remove_offline_validators();

// 		log::debug!(target: LOG_TARGET, "New session called; updated validator set provided.");

// 		Some(Self::validators())
// 	}

// 	fn end_session(_end_index: u32) {}

// 	fn start_session(_start_index: u32) {}
// }

// impl<T: Config> EstimateNextSessionRotation<T::BlockNumber> for Pallet<T> {
// 	fn average_session_length() -> T::BlockNumber {
// 		Zero::zero()
// 	}

// 	fn estimate_current_session_progress(
// 		_now: T::BlockNumber,
// 	) -> (Option<sp_runtime::Permill>, frame_support::dispatch::Weight) {
// 		(None, Zero::zero())
// 	}

// 	fn estimate_next_session_rotation(
// 		_now: T::BlockNumber,
// 	) -> (Option<T::BlockNumber>, frame_support::dispatch::Weight) {
// 		(None, Zero::zero())
// 	}
// }

// // Implementation of Convert trait for mapping ValidatorId with AccountId.
// // This is mainly used to map stash and controller keys.
// // In this pallet, for simplicity, we just return the same AccountId.
// pub struct ValidatorOf<T>(sp_std::marker::PhantomData<T>);

// impl<T: Config> Convert<T::ValidatorId, Option<T::ValidatorId>> for ValidatorOf<T> {
// 	fn convert(account: T::ValidatorId) -> Option<T::ValidatorId> {
// 		Some(account)
// 	}
// }

// impl<T: Config> ValidatorSet<T::AccountId> for Pallet<T> {
// 	type ValidatorId = T::ValidatorId;
// 	type ValidatorIdOf = T::ValidatorIdOf;

// 	fn session_index() -> sp_staking::SessionIndex {
// 		pallet_session::Pallet::<T>::current_index()
// 	}

// 	fn validators() -> Vec<Self::ValidatorId> {
// 		pallet_session::Pallet::<T>::validators()
// 	}
// }

// impl<T: Config> ValidatorSetWithIdentification<T::AccountId> for Pallet<T> {
// 	type Identification = T::ValidatorId;
// 	type IdentificationOf = ValidatorOf<T>;
// }

// // Offence reporting and unresponsiveness management.
// impl<T: Config, O: Offence<(T::AccountId, T::AccountId)>>
// 	ReportOffence<T::AccountId, (T::AccountId, T::AccountId), O> for Pallet<T>
// {
// 	fn report_offence(_reporters: Vec<T::AccountId>, offence: O) -> Result<(), OffenceError> {
// 		let offenders = offence.offenders();

// 		for (v, _) in offenders.into_iter() {
// 			Self::mark_for_removal(v);
// 		}

// 		Ok(())
// 	}

// 	fn is_known_offence(
// 		_offenders: &[(T::AccountId, T::AccountId)],
// 		_time_slot: &O::TimeSlot,
// 	) -> bool {
// 		false
// 	}
// }


//! # Validator Set Pallet
//!
//! The Validator Set Pallet allows addition and removal of authorities/validators via extrinsics (transaction calls), in Substrate-based
//! PoA networks.
//!
//! The pallet uses the Session pallet and implements related traits for session
//! management.

#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;
use sp_runtime::traits::{Convert, Zero};
use pallet_session::{Pallet as Session};

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{dispatch::DispatchResult, pallet_prelude::*};
    use frame_system::pallet_prelude::*;
    use super::*;

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_session::Config {
        /// The Event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Origin for adding or removing a validator.
        type AddRemoveOrigin: EnsureOrigin<Self::Origin>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    // The pallet's storage items.
    #[pallet::storage]
    #[pallet::getter(fn validators)]
    pub type Validators<T: Config> =  StorageValue<_, Vec<T::AccountId>>;

    #[pallet::storage]
    #[pallet::getter(fn flag)]
    pub type Flag<T: Config> =  StorageValue<_, bool>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        // New validator added.
        ValidatorAdded(T::AccountId),

        // Validator removed.
        ValidatorRemoved(T::AccountId),
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        NoValidators,
        ValidatorExists,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub validators: Vec<T::AccountId>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self { validators: Vec::new() }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            Pallet::<T>::initialize_validators(&self.validators);
        }
    }

    #[pallet::call]
    impl<T:Config> Pallet<T> {
        /// Add a new validator using elevated privileges.
        ///
        /// New validator's session keys should be set in session module before calling this.
        ///
        /// The origin can be configured using the `AddRemoveOrigin` type in the host runtime. 
        /// Can also be set to sudo/root.
        #[pallet::weight(0)]
        pub fn add_validator(origin: OriginFor<T>, validator_id: T::AccountId) -> DispatchResult {
            T::AddRemoveOrigin::ensure_origin(origin)?;

            let mut validators: Vec<T::AccountId>;

            if <Validators<T>>::get().is_none() {
                validators = vec![validator_id.clone()];
            } else {
                validators = <Validators<T>>::get().unwrap();
                if !validators.contains(&validator_id.clone()) || validators.len() == 0 {
                    validators.push(validator_id.clone());
                }
            }

            <Validators<T>>::put(validators);

            // Calling rotate_session to queue the new session keys.
            Session::<T>::rotate_session();

            // Triggering rotate session again for the queued keys to take effect.
            Flag::<T>::put(true);

            Self::deposit_event(Event::ValidatorAdded(validator_id));
            Ok(())
        }

        /// Remove a validator using elevated privileges.
        ///
        /// The origin can be configured using the `AddRemoveOrigin` type in the host runtime. 
        /// Can also be set to sudo/root.
        #[pallet::weight(0)]
        pub fn remove_validator(origin: OriginFor<T>, validator_id: T::AccountId) -> DispatchResult {
            T::AddRemoveOrigin::ensure_origin(origin)?;
            let mut validators = <Validators<T>>::get().ok_or(Error::<T>::NoValidators)?;

            // Assuming that this will be a PoA network for enterprise use-cases,
            // the validator count may not be too big; the for loop shouldn't be too heavy.
            // In case the validator count is large, we need to find another way. **TODO**
            for (i, v) in validators.clone().into_iter().enumerate() {
                if v == validator_id {
                    validators.swap_remove(i);
                }
            }
            <Validators<T>>::put(validators);
            
            // Calling rotate_session to queue the new session keys.
            <pallet_session::Pallet<T>>::rotate_session();

            // Triggering rotate session again for the queued keys to take effect.
            Flag::<T>::put(true);

            Self::deposit_event(Event::ValidatorRemoved(validator_id));
            Ok(())
        }

        /// Force rotate session using elevated privileges.
        #[pallet::weight(0)]
        pub fn force_rotate_session(origin: OriginFor<T>) -> DispatchResult {
            T::AddRemoveOrigin::ensure_origin(origin)?;
            
            <pallet_session::Pallet<T>>::rotate_session();
            
            // Triggering rotate session again for any queued keys to take effect.
            // Not sure if double rotate is needed in this scenario. **TODO**
            Flag::<T>::put(true);
            Ok(())
        }
    }
}

impl<T: Config> Pallet<T> {
    fn initialize_validators(validators: &[T::AccountId]) {
            if !validators.is_empty() {
                assert!(<Validators<T>>::get().is_none(), "Validators are already initialized!");
                <Validators<T>>::put(validators);
            }
    }
}

/// Indicates to the session module if the session should be rotated.
/// We set this flag to true when we add/remove a validator.
impl<T: Config> pallet_session::ShouldEndSession<T::BlockNumber> for Pallet<T> {
    fn should_end_session(_now: T::BlockNumber) -> bool {
        Self::flag().unwrap()
    }
}

/// Provides the new set of validators to the session module when session is being rotated.
impl<T: Config> pallet_session::SessionManager<T::AccountId> for Pallet<T> {
    fn new_session(_new_index: u32) -> Option<Vec<T::AccountId>> {
        // Flag is set to false so that the session doesn't keep rotating.
        Flag::<T>::put(false);

        Self::validators()
    }

    fn end_session(_end_index: u32) {}

    fn start_session(_start_index: u32) {}
}

impl<T: Config> frame_support::traits::EstimateNextSessionRotation<T::BlockNumber> for Pallet<T> {
    fn average_session_length() -> T::BlockNumber {
        Zero::zero()
    }

    fn estimate_current_session_progress(_now: T::BlockNumber) -> (Option<sp_runtime::Permill>, frame_support::dispatch::Weight) {
        (None, Zero::zero())
    }

    fn estimate_next_session_rotation(_now: T::BlockNumber) -> (Option<T::BlockNumber>, frame_support::dispatch::Weight) {
        (None, Zero::zero())
    }
}

/// Implementation of Convert trait for mapping ValidatorId with AccountId.
/// This is mainly used to map stash and controller keys.
/// In this module, for simplicity, we just return the same AccountId.
pub struct ValidatorOf<T>(sp_std::marker::PhantomData<T>);

impl<T: Config> Convert<T::AccountId, Option<T::AccountId>> for ValidatorOf<T> {
    fn convert(account: T::AccountId) -> Option<T::AccountId> {
        Some(account)
    }
}

