<#
  set-aumid.ps1 — set System.AppUserModel.ID (PKEY_AppUserModel_ID) on a .lnk.

  WScript.Shell CANNOT set this property, so we go through the shell's
  IShellLink -> IPropertyStore -> IPersistFile.Save path via COM interop.

  GUIDs/PROPERTYKEY from Microsoft's shell sample (see emoacht / Robertof refs).
  PKEY_AppUserModel_ID = {9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3}, PID 5.

  NOTE (important): Claude is an Electron app and calls setAppUserModelId() at
  launch (a process-level AUMID). Windows lets that OVERRIDE the shortcut's AUMID
  for taskbar grouping, so setting this may NOT split profiles into separate
  taskbar groups. We set it anyway (harmless, future-proof, helps Start search /
  jump-list identity). Whether grouping actually splits is verified empirically.

  Usage: powershell -NoProfile -ExecutionPolicy Bypass -File set-aumid.ps1 -Lnk <path> -Aumid <id>
#>
param(
  [Parameter(Mandatory = $true)][string]$Lnk,
  [Parameter(Mandatory = $true)][string]$Aumid
)
$ErrorActionPreference = 'Stop'

Add-Type -Language CSharp -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
namespace CP {
  [StructLayout(LayoutKind.Sequential)]
  public struct PropertyKey {
    public Guid fmtid; public uint pid;
    public PropertyKey(Guid f, uint p) { fmtid = f; pid = p; }
  }
  // Minimal PROPVARIANT: vt at offset 0, data pointer at offset 8 (x86 & x64).
  [StructLayout(LayoutKind.Explicit)]
  public struct PropVariant {
    [FieldOffset(0)] public ushort vt;
    [FieldOffset(8)] public IntPtr p;
  }
  [ComImport, InterfaceType(ComInterfaceType.InterfaceIsIUnknown),
   Guid("886D8EEB-8CF2-4446-8D02-CDBA1DBDCF99")]
  public interface IPropertyStore {
    [PreserveSig] int GetCount(out uint c);
    [PreserveSig] int GetAt(uint i, out PropertyKey k);
    [PreserveSig] int GetValue(ref PropertyKey k, out PropVariant v);
    [PreserveSig] int SetValue(ref PropertyKey k, ref PropVariant v);
    [PreserveSig] int Commit();
  }
  [ComImport, InterfaceType(ComInterfaceType.InterfaceIsIUnknown),
   Guid("0000010b-0000-0000-C000-000000000046")]
  public interface IPersistFile {
    [PreserveSig] int GetClassID(out Guid id);
    [PreserveSig] int IsDirty();
    [PreserveSig] int Load([MarshalAs(UnmanagedType.LPWStr)] string f, uint mode);
    [PreserveSig] int Save([MarshalAs(UnmanagedType.LPWStr)] string f, [MarshalAs(UnmanagedType.Bool)] bool remember);
    [PreserveSig] int SaveCompleted([MarshalAs(UnmanagedType.LPWStr)] string f);
    [PreserveSig] int GetCurFile([MarshalAs(UnmanagedType.LPWStr)] out string f);
  }
  [ComImport, Guid("00021401-0000-0000-C000-000000000046")]
  public class CShellLink { }
  public static class Api {
    const ushort VT_LPWSTR = 31;
    static readonly Guid FMTID = new Guid("9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3");
    [DllImport("ole32.dll")] static extern int PropVariantClear(ref PropVariant pv);
    public static void SetAumid(string lnk, string aumid) {
      IPersistFile pf = (IPersistFile)(new CShellLink());
      pf.Load(lnk, 2); // STGM_READWRITE
      IPropertyStore store = (IPropertyStore)pf;
      PropertyKey key = new PropertyKey(FMTID, 5);
      PropVariant pv = new PropVariant();
      pv.vt = VT_LPWSTR;
      pv.p = Marshal.StringToCoTaskMemUni(aumid);
      try {
        store.SetValue(ref key, ref pv);
        store.Commit();
        pf.Save(lnk, true);
      } finally {
        PropVariantClear(ref pv);
      }
    }
  }
}
'@

[CP.Api]::SetAumid($Lnk, $Aumid)
Write-Output "AUMID set: $Aumid -> $Lnk"
