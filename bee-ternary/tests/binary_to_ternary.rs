// Copyright 2020 IOTA Stiftung
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with
// the License. You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on
// an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and limitations under the License.

use bee_ternary::{
    bigint::{
        common::{BigEndian, U8Repr},
        I384, T242, T243,
    },
    Trits, T1B1,
};

#[test]
fn custom_binary_to_ternary() {
    const BINARY_BE_U8: [u8; 48] = [
        151, 7, 210, 56, 11, 86, 2, 83, 79, 183, 25, 199, 70, 9, 56, 84, 75, 74, 53, 246, 105, 29, 211, 99, 150, 112,
        204, 29, 106, 43, 218, 142, 36, 247, 56, 167, 63, 223, 220, 63, 15, 33, 56, 218, 18, 250, 73, 171,
    ];

    const EXPECTED_TERNARY: [i8; 243] = [
        0, -1, 0, 0, 0, -1, -1, 0, 0, 0, 1, 0, 0, 0, -1, -1, -1, 0, 0, -1, 1, 0, -1, 0, 0, 1, -1, 0, 0, 0, 0, 1, -1, 1,
        -1, 0, 0, 0, -1, -1, 1, 1, 0, -1, 1, -1, 0, -1, 0, 0, -1, -1, 1, 1, -1, -1, 0, 0, 0, -1, 0, -1, -1, 0, 0, 1, 0,
        0, -1, 1, -1, 1, -1, -1, 1, 1, 1, -1, 1, -1, 1, -1, 1, -1, 0, 1, -1, 1, -1, 1, 0, 1, -1, -1, 1, 0, -1, 1, 1, 1,
        1, 0, 1, -1, 0, 1, -1, 1, 1, -1, -1, -1, -1, 0, 1, 1, 0, -1, 1, -1, 0, 1, -1, 0, -1, 0, 0, -1, 0, 0, -1, -1, 0,
        -1, 1, 1, 1, -1, 0, -1, 0, -1, -1, 0, 0, 0, -1, -1, 1, -1, -1, 0, -1, -1, 1, 0, 0, 0, -1, 0, 0, -1, 1, -1, 1,
        1, -1, 1, 0, 1, -1, 1, 0, -1, 0, 0, 1, -1, -1, 1, 1, 1, 1, 1, -1, -1, 1, 0, 0, 0, 0, 1, 1, 0, -1, -1, 1, -1, 0,
        1, -1, -1, -1, 1, 1, -1, 0, -1, -1, 1, -1, 0, 0, 1, 0, 1, 0, -1, 0, 0, -1, 0, -1, -1, 1, 1, -1, -1, 0, 1, -1,
        -1, 1, -1, 1, -1, 0, 0, 0, 0, 1, 1, 0,
    ];

    let i384_be_u8 = I384::<BigEndian, U8Repr>::from_array(BINARY_BE_U8);
    let trit_buf = unsafe { Trits::<T1B1>::from_raw_unchecked(&EXPECTED_TERNARY, EXPECTED_TERNARY.len()).to_buf() };

    let calculated_ternary = T242::from_i384_ignoring_mst(i384_be_u8).into_t243();
    let expected_ternary = T243::from_trit_buf(trit_buf);

    assert_eq!(calculated_ternary, expected_ternary);
}
