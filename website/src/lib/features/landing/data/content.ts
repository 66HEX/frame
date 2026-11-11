export const githubUrl = 'https://github.com/66HEX/frame';
export const releaseUrl = `${githubUrl}/releases/latest`;
export const navigation = [
  { label: 'Features', href: '/#features' },
  { label: 'Workflow', href: '/#workflow' },
  { label: 'Documentation', href: '/docs' }
];
export const downloads = [
  { name: 'macOS', description: 'For Apple Silicon and Intel Macs.', options: [
    { label: 'Apple Silicon', file: 'Frame-aarch64.dmg' },
    { label: 'Intel', file: 'Frame-x86_64.dmg' }
  ] },
  { name: 'Windows', description: 'For 64-bit Windows PCs.', options: [
    { label: 'Download for Windows', file: 'Frame-x86_64.exe' }
  ] },
  { name: 'Linux', description: 'AppImage for x64 and ARM64.', options: [
    { label: 'x64', file: 'Frame-x86_64.AppImage' },
    { label: 'ARM64', file: 'Frame-aarch64.AppImage' }
  ] }
];
export const faqItems = [
  { question: 'Is Frame free?', answer: 'Yes. Frame is free and open source under the GPLv3 license. You can use it, inspect the source, and contribute on GitHub.' },
  { question: 'Do my files leave my computer?', answer: 'Media conversion runs locally with FFmpeg. Your files are not uploaded to a conversion service. Frame connects to GitHub when checking for application updates.' },
  { question: 'Do I need to install FFmpeg?', answer: 'The published desktop packages include FFmpeg and FFprobe. Download Frame for your operating system and follow the installation guide. Building from source has a separate setup step.' },
  { question: 'Can I convert several files at once?', answer: 'Yes. Add files to the queue, choose a preset or adjust each file’s output settings, select an output folder, and start the checked items. You can set the maximum number of concurrent conversions in Settings.' },
  { question: 'Does Frame support hardware encoding?', answer: 'Frame can use VideoToolbox on macOS and NVIDIA NVENC on Windows when the codec and hardware support it. The Linux packages use software encoders. Available options depend on your system.' }
];
