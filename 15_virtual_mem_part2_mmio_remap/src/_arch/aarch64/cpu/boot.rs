// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2021 Andre Richter <andre.o.richter@gmail.com>

//! Architectural boot code.
//!
//! # Orientation
//!
//! Since arch modules are imported into generic modules using the path attribute, the path of this
//! file is:
//!
//! crate::cpu::boot::arch_boot

use crate::{bsp, cpu};
use cortex_a::{asm, regs::*};

//--------------------------------------------------------------------------------------------------
// Private Code
//--------------------------------------------------------------------------------------------------

/// Transition from EL2 to EL1.
///
/// # Safety
///
/// - The HW state of EL1 must be prepared in a sound way.
/// - Exception return from EL2 must must continue execution in EL1 with
///   `runtime_init::runtime_init()`.
/// - We have to hope that the compiler omits any stack pointer usage, because we are not setting up
///   a stack for EL2.
#[inline(always)]
unsafe fn el2_to_el1_transition() -> ! {
    use crate::runtime_init;

    // Enable timer counter registers for EL1.
    CNTHCTL_EL2.write(CNTHCTL_EL2::EL1PCEN::SET + CNTHCTL_EL2::EL1PCTEN::SET);

    // No offset for reading the counters.
    CNTVOFF_EL2.set(0);

    // Set EL1 execution state to AArch64.
    HCR_EL2.write(HCR_EL2::RW::EL1IsAarch64);

    // Set up a simulated exception return.
    //
    // First, fake a saved program status where all interrupts were masked and SP_EL1 was used as a
    // stack pointer.
    SPSR_EL2.write(
        SPSR_EL2::D::Masked
            + SPSR_EL2::A::Masked
            + SPSR_EL2::I::Masked
            + SPSR_EL2::F::Masked
            + SPSR_EL2::M::EL1h,
    );

    // Second, let the link register point to runtime_init().
    ELR_EL2.set(runtime_init::runtime_init as *const () as u64);

    // Set up SP_EL1 (stack pointer), which will be used by EL1 once we "return" to it.
    SP_EL1.set(bsp::memory::phys_boot_core_stack_end().into_usize() as u64);

    // Use `eret` to "return" to EL1. This results in execution of runtime_init() in EL1.
    asm::eret()
}

//--------------------------------------------------------------------------------------------------
// Public Code
//--------------------------------------------------------------------------------------------------

/// The entry of the `kernel` binary.
///
/// The function must be named `_start`, because the linker is looking for this exact name.
///
/// # Safety
///
/// - Linker script must ensure to place this function where it is expected by the target machine.
/// - We have to hope that the compiler omits any stack pointer usage, because we are not setting up
///   a stack for EL2.
#[no_mangle]
pub unsafe fn _start() -> ! {
    // Expect the boot core to start in EL2.
    if (bsp::cpu::BOOT_CORE_ID == cpu::smp::core_id())
        && (CurrentEL.get() == CurrentEL::EL::EL2.value)
    {
        el2_to_el1_transition()
    } else {
        // If not core0, infinitely wait for events.
        cpu::wait_forever()
    }
}
