//! Windows Job Object 兜底（R3）：壳进程死亡（含强杀）时由 OS 自动终止
//! 整个后端进程树。非 Windows 平台为空实现。

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, IsProcessInJob, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JobObjectExtendedLimitInformation, SetInformationJobObject, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
    };
    use std::ptr;

    /// 将指定 PID 的进程加入"壳关闭即杀"的 Job 对象。
    /// 安全设计：若进程已在某个 Job 中且无法再嵌套，则静默跳过（不致命）。
    pub unsafe fn assign_current_job(pid: u32) -> Result<(), String> {
        // 1. 创建 Job（无名字，继承句柄到当前进程）。
        let job = CreateJobObjectW(ptr::null(), ptr::null());
        if job.is_null() {
            return Err("CreateJobObjectW failed".into());
        }

        // 2. 设置 KILL_ON_JOB_CLOSE。
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let ok = SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const _,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        );
        if ok == 0 {
            CloseHandle(job);
            return Err("SetInformationJobObject failed".into());
        }

        // 3. 打开目标进程（尽量少权限）。
        let process = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, pid);
        if process.is_null() {
            CloseHandle(job);
            return Err("OpenProcess failed".into());
        }

        // 4. 若当前进程已在 Job 中且嵌套被禁，分配失败可接受（有 taskkill /T 兜底）。
        let mut in_job: i32 = 0;
        let _ = IsProcessInJob(GetCurrentProcess(), ptr::null_mut(), &mut in_job);
        let assigned = AssignProcessToJobObject(job, process);
        CloseHandle(process);
        if assigned == 0 {
            CloseHandle(job);
            return Err("AssignProcessToJobObject failed".into());
        }
        // Job 句柄在此函数返回后保持打开（继承），进程退出时系统自动关闭 → 杀树。
        Ok(())
    }
}

#[cfg(not(windows))]
mod imp {
    pub unsafe fn assign_current_job(_pid: u32) -> Result<(), String> {
        Ok(())
    }
}

pub use imp::assign_current_job;
