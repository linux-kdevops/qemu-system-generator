// SPDX-License-Identifier: copyleft-next-0.3.1

//! Backends decode a file into a [`crate::plan::Plan`]. The section
//! named after a backend's binary is that binary's command line; QEMU is
//! the first backend.

mod json;
pub mod qemu;
