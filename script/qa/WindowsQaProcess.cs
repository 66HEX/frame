using System;
using System.Collections;
using System.Collections.Generic;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;

// The process joins its own kill-on-close job before its first instruction runs.
// Redirecting to owned files avoids inherited stdout/stderr pipe drain deadlocks.
public static class WindowsQaProcess
{
    [StructLayout(LayoutKind.Sequential)] struct Security { public int Length; public IntPtr Descriptor; public int Inherit; }
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)] struct Startup {
        public int Size; public string Reserved, Desktop, Title;
        public uint X, Y, Width, Height, CharsX, CharsY, Fill, Flags;
        public ushort ShowWindow, ReservedSize; public IntPtr ReservedPointer, Input, Output, Error;
    }
    [StructLayout(LayoutKind.Sequential)] struct ProcessInfo { public IntPtr Process, Thread; public uint ProcessId, ThreadId; }
    [StructLayout(LayoutKind.Sequential)] struct BasicLimits {
        public long ProcessTime, JobTime; public uint Flags;
        public UIntPtr MinWorkingSet, MaxWorkingSet; public uint ActiveProcessLimit;
        public UIntPtr Affinity; public uint PriorityClass, SchedulingClass;
    }
    [StructLayout(LayoutKind.Sequential)] struct IoCounters { public ulong ReadOps, WriteOps, OtherOps, ReadBytes, WriteBytes, OtherBytes; }
    [StructLayout(LayoutKind.Sequential)] struct ExtendedLimits {
        public BasicLimits Basic; public IoCounters Io;
        public UIntPtr ProcessMemory, JobMemory, PeakProcessMemory, PeakJobMemory;
    }
    [StructLayout(LayoutKind.Sequential)] struct Accounting {
        public long UserTime, KernelTime, PeriodUserTime, PeriodKernelTime;
        public uint PageFaults, TotalProcesses, ActiveProcesses, TerminatedProcesses;
    }
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)] static extern IntPtr CreateJobObject(IntPtr attributes, string name);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool SetInformationJobObject(IntPtr job, int kind, ref ExtendedLimits limits, uint length);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool QueryInformationJobObject(IntPtr job, int kind, out Accounting data, uint length, IntPtr returned);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool AssignProcessToJobObject(IntPtr job, IntPtr process);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool TerminateJobObject(IntPtr job, uint code);
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)] static extern IntPtr CreateFile(string path, uint access, uint share, ref Security security, uint disposition, uint attributes, IntPtr template);
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)] static extern bool CreateProcess(string app, StringBuilder command, IntPtr processSecurity, IntPtr threadSecurity, bool inherit, uint flags, IntPtr environment, string directory, ref Startup startup, out ProcessInfo process);
    [DllImport("kernel32.dll", SetLastError=true)] static extern uint ResumeThread(IntPtr thread);
    [DllImport("kernel32.dll", SetLastError=true)] static extern uint WaitForSingleObject(IntPtr handle, uint milliseconds);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool GetExitCodeProcess(IntPtr process, out uint code);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool TerminateProcess(IntPtr process, uint code);
    [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr handle);
    static void Check(bool ok) { if (!ok) throw new Win32Exception(Marshal.GetLastWin32Error()); }
    static void Close(ref IntPtr handle) { if (handle != IntPtr.Zero && handle != new IntPtr(-1)) CloseHandle(handle); handle=IntPtr.Zero; }
    static IntPtr OpenFile(string path, uint access, uint disposition) {
        Security security=new Security(); security.Length=Marshal.SizeOf(typeof(Security)); security.Inherit=1;
        IntPtr file=CreateFile(path, access, 3, ref security, disposition, 128, IntPtr.Zero);
        if (file==new IntPtr(-1)) throw new Win32Exception(Marshal.GetLastWin32Error());
        return file;
    }
    static void StopJob(IntPtr job, uint code) {
        Check(TerminateJobObject(job,code));
        for(int elapsed=0; elapsed<5000; elapsed+=20) {
            Accounting data;
            Check(QueryInformationJobObject(job,1,out data,(uint)Marshal.SizeOf(typeof(Accounting)),IntPtr.Zero));
            if(data.ActiveProcesses==0) return;
            Thread.Sleep(20);
        }
        throw new TimeoutException("Owned QA job did not stop within cleanup deadline");
    }
    public static int Run(string executable, string arguments, string directory, string stdout, string stderr, string ffmpeg, string ffprobe, int timeoutMs) {
        bool timedOut;
        return Run(executable,arguments,directory,stdout,stderr,ffmpeg,ffprobe,timeoutMs,out timedOut);
    }
    public static int Run(string executable, string arguments, string directory, string stdout, string stderr, string ffmpeg, string ffprobe, int timeoutMs, out bool timedOut) {
        timedOut=false;
        if(timeoutMs<=0) throw new ArgumentOutOfRangeException("timeoutMs");
        IntPtr job=IntPtr.Zero, output=IntPtr.Zero, error=IntPtr.Zero, input=IntPtr.Zero, environment=IntPtr.Zero;
        ProcessInfo process=new ProcessInfo(); bool assigned=false;
        try {
            job=CreateJobObject(IntPtr.Zero,null); Check(job!=IntPtr.Zero);
            ExtendedLimits limits=new ExtendedLimits(); limits.Basic.Flags=0x2000; // KILL_ON_JOB_CLOSE
            Check(SetInformationJobObject(job,9,ref limits,(uint)Marshal.SizeOf(typeof(ExtendedLimits))));
            output=OpenFile(stdout,0x40000000,2); error=OpenFile(stderr,0x40000000,2); input=OpenFile("NUL",0x80000000,3);
            SortedDictionary<string,string> variables=new SortedDictionary<string,string>(StringComparer.OrdinalIgnoreCase);
            foreach(DictionaryEntry item in Environment.GetEnvironmentVariables()) variables[(string)item.Key]=(string)item.Value;
            variables["FRAME_FFMPEG_PATH"]=ffmpeg; variables["FRAME_FFPROBE_PATH"]=ffprobe;
            StringBuilder block=new StringBuilder(); foreach(KeyValuePair<string,string> item in variables) block.Append(item.Key).Append('=').Append(item.Value).Append('\0'); block.Append('\0');
            environment=Marshal.StringToHGlobalUni(block.ToString());
            Startup startup=new Startup(); startup.Size=Marshal.SizeOf(typeof(Startup)); startup.Flags=0x100;
            startup.Input=input; startup.Output=output; startup.Error=error;
            StringBuilder command=new StringBuilder("\""+executable+"\" "+arguments);
            Check(CreateProcess(executable,command,IntPtr.Zero,IntPtr.Zero,true,0x08000404,environment,directory,ref startup,out process)); // NO_WINDOW | UNICODE_ENVIRONMENT | SUSPENDED
            Check(AssignProcessToJobObject(job,process.Process)); assigned=true;
            Check(ResumeThread(process.Thread)!=UInt32.MaxValue);
            Close(ref process.Thread);
            uint wait=WaitForSingleObject(process.Process,(uint)timeoutMs);
            if(wait==0x102) { timedOut=true; Close(ref process.Process); StopJob(job,124); return 124; }
            Check(wait==0);
            uint code; Check(GetExitCodeProcess(process.Process,out code));
            // ActiveProcesses drops only after process references are released.
            // The job still owns the tree while its termination is verified.
            Close(ref process.Process);
            // Parent exit does not release ownership of descendants. Stop them before reading logs.
            StopJob(job,0);
            return unchecked((int)code);
        } finally {
            if(process.Process!=IntPtr.Zero && !assigned) { TerminateProcess(process.Process,125); WaitForSingleObject(process.Process,5000); }
            Close(ref job); Close(ref process.Thread); Close(ref process.Process);
            Close(ref input); Close(ref output); Close(ref error);
            if(environment!=IntPtr.Zero) Marshal.FreeHGlobal(environment);
        }
    }
}
