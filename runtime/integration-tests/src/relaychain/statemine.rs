use crate::relaychain::kusama_test_net::*;
use crate::setup::*;
use cumulus_primitives_core::ParaId;

use frame_support::assert_ok;
pub use orml_traits::GetByKey;
use polkadot_parachain::primitives::Sibling;
use xcm::v1::{Junction, MultiLocation};
use xcm_emulator::TestExt;

pub const UNIT: Balance = 1_000_000_000_000;
pub const TEN: Balance = 10_000_000_000_000;
pub const FEE_WEIGHT: Balance = 4_000_000_000;
pub const FEE_STATEMINE: Balance = 15_540_916;

fn init_statemine_xcm_interface() {
	let xcm_operation =
		module_xcm_interface::XcmInterfaceOperation::ParachainFee(Box::new((1, Parachain(1000)).into()));
	assert_ok!(<module_xcm_interface::Pallet<Runtime>>::update_xcm_dest_weight_and_fee(
		Origin::root(),
		vec![(xcm_operation.clone(), Some(4_000_000_000), Some(4_000_000_000),)],
	));
	System::assert_has_event(Event::XcmInterface(module_xcm_interface::Event::XcmDestWeightUpdated {
		xcm_operation: xcm_operation.clone(),
		new_xcm_dest_weight: 4_000_000_000,
	}));
	System::assert_has_event(Event::XcmInterface(module_xcm_interface::Event::XcmFeeUpdated {
		xcm_operation,
		new_xcm_dest_weight: 4_000_000_000,
	}));
}

#[test]
fn statemine_min_xcm_fee_matched() {
	Statemine::execute_with(|| {
		use frame_support::weights::{IdentityFee, WeightToFee};

		init_statemine_xcm_interface();
		let weight = FEE_WEIGHT as u64;

		let fee: Balance = IdentityFee::weight_to_fee(&weight);
		let statemine: MultiLocation = (1, Parachain(parachains::statemine::ID)).into();
		let bifrost: MultiLocation = (1, Parachain(parachains::bifrost::ID)).into();

		let statemine_fee: u128 = ParachainMinFee::get(&statemine).unwrap();
		assert_eq!(fee, statemine_fee);

		let bifrost_fee: Option<u128> = ParachainMinFee::get(&bifrost);
		assert_eq!(None, bifrost_fee);
	});
}

#[test]
fn transfer_from_relay_chain() {
	KusamaNet::execute_with(|| {
		assert_ok!(kusama_runtime::XcmPallet::reserve_transfer_assets(
			kusama_runtime::Origin::signed(ALICE.into()),
			Box::new(Parachain(1000).into().into()),
			Box::new(
				Junction::AccountId32 {
					id: BOB,
					network: NetworkId::Any
				}
				.into()
				.into()
			),
			Box::new((Here, dollar(KSM)).into()),
			0
		));
	});

	Statemine::execute_with(|| {
		assert_eq!(
			dollar(KSM) - FEE_STATEMINE,
			Balances::free_balance(&AccountId::from(BOB))
		);
	});
}

#[test]
fn karura_statemine_transfer_works() {
	TestNet::reset();
	let para_2000: AccountId = Sibling::from(2000).into_account_truncating_truncating();
	let child_2000: AccountId = ParaId::from(2000).into_account_truncating_truncating();
	let child_1000: AccountId = ParaId::from(1000).into_account_truncating_truncating();

	// minimum asset should be: FEE_WEIGHT+FEE_KUSAMA+max(KUSAMA_ED,STATEMINE_ED+FEE_STATEMINE).
	// but due to current half fee, sender asset should at lease: FEE_WEIGHT + 2 * FEE_KUSAMA
	let asset = FEE_WEIGHT + 2 * 31_488_122;

	statemine_side(UNIT);

	KusamaNet::execute_with(|| {
		let _ = kusama_runtime::Balances::make_free_balance_be(&child_2000, TEN);
		assert_eq!(0, kusama_runtime::Balances::free_balance(&child_1000));
	});

	pioneer_side(asset);

	KusamaNet::execute_with(|| {
		assert_eq!(
			TEN - (asset - FEE_WEIGHT),
			kusama_runtime::Balances::free_balance(&child_2000)
		);
		assert_eq!(33_333_334, kusama_runtime::Balances::free_balance(&child_1000));
	});

	Statemine::execute_with(|| {
		use statemine_runtime::*;
		// Pioneer send back custom asset to Statemine, ensure recipient got custom asset
		assert_eq!(UNIT, Assets::balance(0, &AccountId::from(BOB)));
		// and withdraw sibling parachain sovereign account
		assert_eq!(9 * UNIT, Assets::balance(0, &para_2000));

		assert_eq!(
			UNIT + FEE_WEIGHT - FEE_STATEMINE,
			Balances::free_balance(&AccountId::from(BOB))
		);
		assert_eq!(996_017_792_418, Balances::free_balance(&para_2000));
	});
}

// transfer custom asset from Pioneer to Statemine
fn pioneer_side(fee_amount: u128) {
	Pioneer::execute_with(|| {
		init_statemine_xcm_interface();

		assert_eq!(
			9_999_906_760_000,
			Tokens::free_balance(CurrencyId::ForeignAsset(0), &AccountId::from(BOB))
		);
		// ensure sender has enough KSM balance to be charged as fee
		assert_ok!(Tokens::deposit(KSM, &AccountId::from(BOB), TEN));

		assert_ok!(XTokens::transfer_multicurrencies(
			Origin::signed(BOB.into()),
			vec![(CurrencyId::ForeignAsset(0), UNIT), (KSM, fee_amount)],
			1,
			Box::new(
				MultiLocation::new(
					1,
					X2(
						Parachain(1000),
						Junction::AccountId32 {
							network: NetworkId::Any,
							id: BOB.into(),
						}
					)
				)
				.into()
			),
			FEE_WEIGHT as u64
		));

		assert_eq!(
			8_999_906_760_000,
			Tokens::free_balance(CurrencyId::ForeignAsset(0), &AccountId::from(BOB))
		);
		assert_eq!(TEN - fee_amount, Tokens::free_balance(KSM, &AccountId::from(BOB)));
	});
}

// transfer custom asset from Statemine to Pioneer
fn statemine_side(para_2000_init_amount: u128) {
	register_asset();

	let para_acc: AccountId = Sibling::from(2000).into_account_truncating_truncating();

	Statemine::execute_with(|| {
		use statemine_runtime::*;

		let origin = Origin::signed(ALICE.into());
		Balances::make_free_balance_be(&ALICE.into(), TEN);
		Balances::make_free_balance_be(&BOB.into(), UNIT);

		// create custom asset cost 1 KSM
		assert_ok!(Assets::create(
			origin.clone(),
			0,
			MultiAddress::Id(ALICE.into()),
			UNIT / 100
		));
		assert_eq!(9 * UNIT, Balances::free_balance(&AccountId::from(ALICE)));

		assert_ok!(Assets::mint(
			origin.clone(),
			0,
			MultiAddress::Id(ALICE.into()),
			1000 * UNIT
		));

		// need to have some KSM to be able to receive user assets
		Balances::make_free_balance_be(&para_acc, para_2000_init_amount);

		assert_ok!(PolkadotXcm::reserve_transfer_assets(
			origin.clone(),
			Box::new(MultiLocation::new(1, X1(Parachain(2000))).into()),
			Box::new(
				Junction::AccountId32 {
					id: BOB,
					network: NetworkId::Any
				}
				.into()
				.into()
			),
			Box::new((X2(PalletInstance(50), GeneralIndex(0)), TEN).into()),
			0
		));

		assert_eq!(0, Assets::balance(0, &AccountId::from(BOB)));

		assert_eq!(TEN, Assets::balance(0, &para_acc));
		// the KSM balance of sibling parachain sovereign account is not changed
		assert_eq!(para_2000_init_amount, Balances::free_balance(&para_acc));
	});

	// Rerun the Statemine::execute to actually send the egress message via XCM
	Statemine::execute_with(|| {});
}

fn register_asset() {
	Pioneer::execute_with(|| {
		// register foreign asset
		assert_ok!(AssetRegistry::register_foreign_asset(
			Origin::root(),
			Box::new(MultiLocation::new(1, X3(Parachain(1000), PalletInstance(50), GeneralIndex(0))).into()),
			Box::new(AssetMetadata {
				name: b"Sibling Token".to_vec(),
				symbol: b"ST".to_vec(),
				decimals: 10,
				minimal_balance: Balances::minimum_balance() / 100, // 10%
			})
		));
	});
}
