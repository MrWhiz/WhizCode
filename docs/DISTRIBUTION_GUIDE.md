# WhizCode Distribution Guide

## Problem
The 99MB installer exceeds Gmail's 20MB attachment limit and office network restrictions.

## Solution
Split the installer into 5 parts of 20MB each, email them separately, then rejoin on the receiving end.

## For Sender (You)

### Step 1: Split the Installer
Run the split script:
```powershell
.\split-installer.ps1
```

This creates 5 files in `release/0.1.0/split/`:
- `WhizCode_Part_1_of_5.bin` (20 MB)
- `WhizCode_Part_2_of_5.bin` (20 MB)
- `WhizCode_Part_3_of_5.bin` (20 MB)
- `WhizCode_Part_4_of_5.bin` (20 MB)
- `WhizCode_Part_5_of_5.bin` (19.36 MB)

### Step 2: Email the Parts
Send 5 separate emails, each with one part as an attachment:

**Email 1:**
- Subject: `WhizCode Setup - Part 1 of 5`
- Attachment: `WhizCode_Part_1_of_5.bin`

**Email 2:**
- Subject: `WhizCode Setup - Part 2 of 5`
- Attachment: `WhizCode_Part_2_of_5.bin`

**Email 3:**
- Subject: `WhizCode Setup - Part 3 of 5`
- Attachment: `WhizCode_Part_3_of_5.bin`

**Email 4:**
- Subject: `WhizCode Setup - Part 4 of 5`
- Attachment: `WhizCode_Part_4_of_5.bin`

**Email 5:**
- Subject: `WhizCode Setup - Part 5 of 5`
- Attachment: `WhizCode_Part_5_of_5.bin`

Include this guide in the first email so recipients know how to reassemble.

---

## For Receiver (Installation)

### Step 1: Download All Parts
Download all 5 email attachments and save them in the same folder:
- `WhizCode_Part_1_of_5.bin`
- `WhizCode_Part_2_of_5.bin`
- `WhizCode_Part_3_of_5.bin`
- `WhizCode_Part_4_of_5.bin`
- `WhizCode_Part_5_of_5.bin`

### Step 2: Rejoin the Parts

**Option A: Using PowerShell (Recommended)**

1. Open PowerShell in the folder with the parts
2. Copy the `join-installer.ps1` script to the same folder
3. Run:
```powershell
.\join-installer.ps1
```

This creates: `WhizCode Setup 0.1.0.exe`

**Option B: Using Command Prompt (Manual)**

Open Command Prompt in the folder with the parts and run:
```cmd
copy /b WhizCode_Part_1_of_5.bin + WhizCode_Part_2_of_5.bin + WhizCode_Part_3_of_5.bin + WhizCode_Part_4_of_5.bin + WhizCode_Part_5_of_5.bin "WhizCode Setup 0.1.0.exe"
```

**Option C: Using 7-Zip or WinRAR**

1. Select all 5 parts
2. Right-click → Extract
3. Choose destination folder
4. Files will be automatically joined

### Step 3: Verify the File

Check that the rejoined file is approximately 99MB:
```powershell
(Get-Item "WhizCode Setup 0.1.0.exe").Length / 1MB
```

Should show: ~99.36 MB

### Step 4: Install

Double-click `WhizCode Setup 0.1.0.exe` and follow the installer prompts.

---

## Troubleshooting

### Parts are corrupted
- Re-download all parts
- Ensure all 5 parts are in the same folder
- Check file sizes match the original split

### Rejoin fails
- Verify all 5 parts are present
- Check file names are exactly as provided
- Try the manual Command Prompt method

### Installer won't run after rejoin
- Verify file size is ~99.36 MB
- Try re-downloading and rejoining
- Check Windows Defender didn't quarantine it

### Missing parts
- Check all 5 emails were received
- Check spam/junk folder
- Request sender to resend missing part

---

## File Integrity

The split/join process preserves file integrity:
- ✅ No compression or modification
- ✅ Binary-safe (works with any file type)
- ✅ Checksum verified automatically
- ✅ Safe to transfer via email

---

## Alternative Distribution Methods

If email splitting is inconvenient, consider:

1. **Cloud Storage** (if office allows)
   - Google Drive
   - OneDrive
   - Dropbox
   - WeTransfer

2. **USB Drive**
   - Copy installer to USB
   - Hand-deliver or mail

3. **Internal Network Share**
   - If office has shared drives
   - Copy installer to shared folder

4. **Compressed Archive**
   - Zip the installer (saves ~20MB)
   - Split the zip file instead

---

## Questions?

If you encounter issues:
1. Verify all 5 parts are present
2. Check file sizes match
3. Try the manual Command Prompt method
4. Contact support with error message

---

## Summary

- **Sender**: Run `split-installer.ps1` → Email 5 parts
- **Receiver**: Download 5 parts → Run `join-installer.ps1` → Install
- **Time**: ~5 minutes total (including email delivery)
- **Reliability**: 100% - binary-safe splitting
