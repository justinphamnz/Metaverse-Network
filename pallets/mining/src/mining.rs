// This file is part of Bit.Country.

// Copyright (C) 2020-2021 Bit.Country.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::traits::Zero;
use sp_runtime::{Perbill, RuntimeDebug};

use primitives::estate::Estate;

// Helper methods to compute the issuance rate for undeployed land.
use crate::pallet::{Config, Pallet};

const SECONDS_PER_YEAR: u32 = 31557600;
const SECONDS_PER_BLOCK: u32 = 12;
const BLOCKS_PER_YEAR: u32 = SECONDS_PER_YEAR / SECONDS_PER_BLOCK;

fn rounds_per_year<T: Config>() -> u32 {
	let blocks_per_round = <Pallet<T>>::issuance_round().length;
	BLOCKS_PER_YEAR / blocks_per_round
}

/// Compute round issuance range from round inflation range and current total issuance
pub fn round_issuance_range<T: Config>(config: MiningResourceRateInfo) -> Range<u64> {
	// Get total round per year
	let total_round_per_year = rounds_per_year::<T>();
	// Initial minting ratio per land unit
	let minting_ratio = config.ratio;
	// Get total deployed land unit circulating
	let total_land_unit_circulating = T::EstateHandler::get_total_land_units();

	let issuance_per_round = total_land_unit_circulating
		.checked_mul(minting_ratio)
		.unwrap_or(Zero::zero());

	let land_allocation = issuance_per_round
		.checked_mul(config.land_reward.into())
		.unwrap_or(issuance_per_round)
		.checked_div(100u64)
		.unwrap();

	let metaverse_allocation = issuance_per_round
		.checked_mul(config.metaverse_reward.into())
		.unwrap_or(issuance_per_round)
		.checked_div(100u64)
		.unwrap();

	// Return range - could implement more cases in the future.
	Range {
		min: issuance_per_round,
		ideal: issuance_per_round,
		max: issuance_per_round,
		land_allocation: land_allocation,
		metaverse_allocation: metaverse_allocation,
	}
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Eq, PartialEq, Clone, Copy, Encode, Decode, Default, RuntimeDebug, TypeInfo)]
pub struct Range<T> {
	pub min: T,
	pub ideal: T,
	pub max: T,
	pub land_allocation: T,
	pub metaverse_allocation: T,
}

impl<T: Ord> Range<T> {
	pub fn is_valid(&self) -> bool {
		self.max >= self.ideal && self.ideal >= self.min
	}
}

impl<T: Ord + Copy> From<T> for Range<T> {
	fn from(other: T) -> Range<T> {
		Range {
			min: other,
			ideal: other,
			max: other,
			land_allocation: other,
			metaverse_allocation: other,
		}
	}
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Eq, PartialEq, Clone, Encode, Decode, Default, RuntimeDebug, TypeInfo)]
pub struct MiningResourceRateInfo {
	/// kBIT and Land unit ratio
	pub ratio: u64,
	/// land staking reward percentage
	pub land_reward: u32,
	/// metaverse staking reward percentage
	pub metaverse_reward: u32,
}

impl MiningResourceRateInfo {
	pub fn new<T: Config>(ratio: u64, land_reward: u32, metaverse_reward: u32) -> MiningResourceRateInfo {
		MiningResourceRateInfo {
			ratio,
			land_reward,
			metaverse_reward,
		}
	}

	/// kBIT and Land unit ratio
	pub fn set_ratio(&mut self, ratio: u64) {
		self.ratio = ratio;
	}

	/// Set land reward
	pub fn set_land_reward(&mut self, land_reward: u32) {
		self.land_reward = land_reward;
	}

	/// Set metaverse reward
	pub fn set_metaverse_reward(&mut self, metaverse_reward: u32) {
		self.metaverse_reward = metaverse_reward;
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	/// Compute round issuance range from round inflation range and current total issuance
	pub fn mock_round_issuance_per_year(config: MiningResourceRateInfo, land_unit_circulation: u64) -> Range<u64> {
		let issuance_per_round = land_unit_circulation.checked_mul(config.ratio).unwrap_or(Zero::zero());

		let land_allocation = issuance_per_round
			.checked_mul(config.land_reward.into())
			.unwrap_or(issuance_per_round)
			.checked_div(100u64)
			.unwrap();

		let metaverse_allocation = issuance_per_round
			.checked_mul(config.metaverse_reward.into())
			.unwrap_or(issuance_per_round)
			.checked_div(100u64)
			.unwrap();

		// Return range - could implement more cases in the future.
		Range {
			min: issuance_per_round,
			ideal: issuance_per_round,
			max: issuance_per_round,
			land_allocation: land_allocation,
			metaverse_allocation: metaverse_allocation,
		}
	}

	#[test]
	fn simple_round_issuance() {
		// 10 BIT/Land unit minting ratio for 2_000 land unit = 2_000_000 minted over the year
		// let's assume there are 10 periods in a year
		// => mint 2_000_000 over 10 periods => 20_000 minted per period

		let mock_config: MiningResourceRateInfo = MiningResourceRateInfo {
			ratio: 10,
			land_reward: 20,
			metaverse_reward: 80,
		};

		let round_issuance = mock_round_issuance_per_year(mock_config, 2_000);

		// make sure 20_000 land unit deploy per period
		assert_eq!(round_issuance.min, 20_000);
		assert_eq!(round_issuance.ideal, 20_000);
		assert_eq!(round_issuance.max, 20_000);
		assert_eq!(round_issuance.land_allocation, 4_000);
		assert_eq!(round_issuance.metaverse_allocation, 16_000);
	}
}
