// SPDX-License-Identifier: copyleft-next-0.3.1

//! Launchers turn a [`crate::plan::Plan`] into the files a service
//! manager runs. systemd is the first launcher.

pub mod systemd;
