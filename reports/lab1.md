# Lab 1 Report

## 实现综述

本次实验主要实现了 `sys_trace` 系统调用，用于追踪当前应用程序对于系统调用的使用情况，并且能够读取、写入指定的内存区域。为了实现这一功能，设计一个专用的审计数据结构 `SysCallAudit` 针对某一系统调用发起次数的计数。

针对每个任务采用按任务计数机制，在每个任务控制块（`TaskControlBlock`）中添加 `syscall_audit` 字段，为每个任务维护独立的系统调用审计数组。在系统调用的管理函数 `syscall` 中，调用 `update_current_task_syscall_count` 函数更新当前任务的系统调用计数。在 `sys_trace` 系统调用中，通过 `get_current_task_syscall_count` 函数获取当前任务的特定系统调用计数，实现按当前任务进行的统计。

## 简答作业
### 第一题
***正确进入 U 态后，程序的特征还应有：使用 S 态特权指令，访问 S 态寄存器后会报错。 请同学们可以自行测试这些内容（运行 三个 bad 测例 (ch2b_bad_\*.rs) ）， 描述程序出错行为，同时注意注明你使用的 sbi 及其版本。***

### SBI 版本信息
- **RustSBI 版本**: 0.3.0-alpha.2
- **实现版本**: RustSBI-QEMU Version 0.2.0-alpha.2
- **平台**: riscv-virtio,qemu
- **支持 SBI 标准**: RISC-V SBI v1.0.0

### 三个 bad 测例的错误行为分析

#### 1. ch2b_bad_address.rs - 非法内存访问
**测试内容**: 程序尝试向地址 0x0 写入数据
**错误行为**:
```
[kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003a4, kernel killed it.
```
**分析**: 系统正确检测到了页面错误异常，地址 0x0 是非法访问地址，硬件 MMU 保护机制触发了 PageFault 异常，内核捕获异常并终止了应用程序。

#### 2. ch2b_bad_instructions.rs - 特权指令测试
**测试内容**: 程序在 U 态尝试执行 S 态特权指令 `sret`
**错误行为**:
```
[kernel] IllegalInstruction in application, kernel killed it.
```
**分析**: 系统正确检测到了非法指令异常。`sret` 是 S 态特权指令，在 U 态执行时会触发 IllegalInstruction 异常，RISC-V 硬件特权级别保护机制有效工作。

#### 3. ch2b_bad_register.rs - 特权寄存器访问测试
**测试内容**: 程序在 U 态尝试读取 S 态特权寄存器 `sstatus`
**错误行为**:
```
[kernel] IllegalInstruction in application, kernel killed it.
```
**分析**: 系统正确检测到了非法指令异常。CSR 寄存器 `sstatus` 是 S 态特权寄存器，U 态程序无权访问，硬件会将此类访问识别为非法指令并触发异常。

### 结论
测试结果表明，RustSBI 和 rCore 操作系统正确实现了 RISC-V 的特权级别保护机制：
1. Risc-V 硬件机制防止对非法地址的访问
2. 硬件正确阻止 U 态程序执行 S 态特权指令
3. 硬件正确阻止 U 态程序访问 S 态特权寄存器
4. 内核能够根据 Risc-V 提供的异常编码调用对应的处理函数，正确处理这些异常并终止违规程序

## 第二题
***深入理解 trap.S 中两个函数 __alltraps 和 __restore 的作用，并回答如下问题:***

### 1. L40：刚进入 __restore 时，sp 代表了什么值。请指出 __restore 的两种使用情景。

**L40 刚进入 __restore 时，sp 的含义**：
sp 指向内核栈中已分配的 TrapContext 结构的起始地址。此时 sp 指向的内存区域包含了需要恢复的所有寄存器状态，包括通用寄存器 x0-x31（x0，x2,x4特殊处理）、sstatus 和 sepc。

**__restore 的两种使用情景**：
1. **用户进程从异常或中断返回**：当用户程序触发异常或中断，经过 `__alltraps` → `trap_handler` 处理后，通过 `__restore` 恢复用户程序的执行上下文并返回用户态。
2. **任务切换或首次进入用户态**：通过 `run_first_task()` 或任务调度时，使用预设的 TrapContext 切换到新的用户程序或者加载先前已经保存的上下文执行。

### 2. L43-L48：这几行汇编代码特殊处理了哪些寄存器？这些寄存器的值对于进入用户态有何意义？

```assembly
ld t0, 32*8(sp)    # 加载 sstatus， Supervisor 状态寄存器
ld t1, 33*8(sp)    # 加载 sepc， Supervisor 异常程序计数器
ld t2, 2*8(sp)     # 加载用户栈指针 sscratch， Supervisor Scratch Register
csrw sstatus, t0   # 恢复 sstatus
csrw sepc, t1      # 恢复 sepc
csrw sscratch, t2  # 恢复 sscratch
```

**特殊处理的寄存器及意义**：
- **sstatus**：Supervisor 状态寄存器，控制特权级别。其中 SPP 位决定 `sret` 指令执行后返回到 U 态还是 S 态
- **sepc**：Supervisor 异常程序计数器，存储异常返回后要执行的指令地址
- **sscratch**：记录异常发生时的程序状态，此处用于保存用户栈指针，作为内核栈和用户栈切换的中转

这些寄存器是 RISC-V 特权级别切换的关键，此处在进入用户态前正确设置，以确保程序能在用户态继续执行。

### 3. L50-L56：为何跳过了 x2 和 x4？

```assembly
ld x1, 1*8(sp)
ld x3, 3*8(sp)
.set n, 5
.rept 27
   LOAD_GP %n
   .set n, n+1
.endr
```

**跳过原因**：
- **x2 (sp)**：栈指针寄存器，因为此时其指向内核栈，并且需要借助内核栈完成后续的用户态上下文恢复操作，不能提前恢复
- **x4 (tp)**：线程指针寄存器，用户态通常不使用该寄存器，跳过来提高效率

### 4. L60：该指令之后，sp 和 sscratch 中的值分别有什么意义？

```assembly
csrrw sp, sscratch, sp
```

**执行后的状态**：
- **sp**：指向用户栈，恢复为用户程序的栈指针
- **sscratch**：指向内核栈顶，为下次异常处理做准备

这条指令交换了栈指针和当前指向内核栈的指针，交换了硬件执行的环境，实现了内核栈和用户栈的切换。

### 5. __restore 中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？

**状态切换指令**：L61 的 `sret` 指令

**进入用户态的原因**：
`sret` 是 RISC-V 的特权指令，执行时会：
1. 将当前特权级别设置为 sstatus.SPP 字段的值 (User 模式)
2. 跳转到 sepc 寄存器指向的地址继续执行

由于在 TrapContext 初始化时设置了 `sstatus.set_spp(SPP::User);`，因此 `sret` 会根据这里预设的 `sstatus.SPP` 切换到用户态。

### 6. L13：该指令之后，sp 和 sscratch 中的值分别有什么意义？

```assembly
csrrw sp, sscratch, sp    # __alltraps 中的 L13
```

**执行后的状态**：
- **sp**：指向内核栈，为保存用户程序上下文做准备
- **sscratch**：保存用户栈指针，确保稍后能恢复用户程序的栈状态

借此来实现内核栈和用户栈的切换，为异常处理做好准备。

### 7. 从 U 态进入 S 态是哪一条指令发生的？

从 U 态进入 S 态不是由程序中的某条指令直接实现的，而是由 RISC-V 硬件机制触发。

当用户程序执行 `ecall` 指令、触发异常或中断时，硬件自动将特权级别从 U 态切换到 S 态，同时将程序计数器设置为 stvec 寄存器的值 (指向 `__alltraps`)，并将异常前的 PC 保存到 sepc 寄存器，这是由硬件自动完成的，不需要程序显式调用某条指令。而具体进行这项配置则是在 `init` 函数中完成：
```rust
pub fn init() {
    unsafe extern "C" { safe fn __alltraps(); }
    unsafe {
        stvec::write(__alltraps as usize, TrapMode::Direct);
    }
}
```