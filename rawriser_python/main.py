import tkinter as tk
from tkinter import filedialog, ttk
import rawpy
import imageio
import os
from pathlib import Path
import threading

class RawConverter:
    def __init__(self, root):
        self.root = root
        self.root.title("RAW to JPEG Converter")
        self.root.geometry("500x300")
        
        # Source folder selection
        tk.Label(root, text="Source Folder:").pack(pady=5)
        self.source_frame = tk.Frame(root)
        self.source_frame.pack(fill='x', padx=5)
        self.source_entry = tk.Entry(self.source_frame)
        self.source_entry.pack(side='left', fill='x', expand=True)
        tk.Button(self.source_frame, text="Browse", command=self.browse_source).pack(side='right', padx=5)
        
        # Destination folder selection
        tk.Label(root, text="Destination Folder:").pack(pady=5)
        self.dest_frame = tk.Frame(root)
        self.dest_frame.pack(fill='x', padx=5)
        self.dest_entry = tk.Entry(self.dest_frame)
        self.dest_entry.pack(side='left', fill='x', expand=True)
        tk.Button(self.dest_frame, text="Browse", command=self.browse_dest).pack(side='right', padx=5)
        
        # Convert button
        self.convert_btn = tk.Button(root, text="Convert", command=self.start_conversion)
        self.convert_btn.pack(pady=20)
        
        # Progress bar
        self.progress = ttk.Progressbar(root, length=300, mode='determinate')
        self.progress.pack(pady=10)
        
        # Status label
        self.status_label = tk.Label(root, text="Ready")
        self.status_label.pack(pady=5)

    def browse_source(self):
        folder = filedialog.askdirectory()
        if folder:
            self.source_entry.delete(0, tk.END)
            self.source_entry.insert(0, folder)

    def browse_dest(self):
        folder = filedialog.askdirectory()
        if folder:
            self.dest_entry.delete(0, tk.END)
            self.dest_entry.insert(0, folder)

    def convert_raw_to_jpeg(self, raw_path, jpeg_path):
        with rawpy.imread(raw_path) as raw:
            rgb = raw.postprocess(use_camera_wb=True)
            imageio.imsave(jpeg_path, rgb)

    def start_conversion(self):
        source = self.source_entry.get()
        dest = self.dest_entry.get()
        
        if not source or not dest:
            self.status_label.config(text="Please select both folders")
            return
            
        self.convert_btn.config(state='disabled')
        threading.Thread(target=self.convert_files, args=(source, dest)).start()

    def convert_files(self, source, dest):
        raw_extensions = ('.arw', '.nef', '.cr2', '.crw', '.orf', '.rw2')
        files = [f for f in Path(source).glob('*') if f.suffix.lower() in raw_extensions]
        total_files = len(files)
        
        if total_files == 0:
            self.root.after(0, lambda: self.status_label.config(text="No RAW files found"))
            self.root.after(0, lambda: self.convert_btn.config(state='normal'))
            return
            
        for i, file in enumerate(files, 1):
            try:
                output_path = Path(dest) / f"{file.stem}.jpg"
                self.convert_raw_to_jpeg(str(file), str(output_path))
                progress = (i / total_files) * 100
                self.root.after(0, lambda p=progress: self.progress.config(value=p))
                self.root.after(0, lambda c=i, t=total_files: 
                    self.status_label.config(text=f"Converting: {c}/{t} files"))
            except Exception as e:
                print(f"Error converting {file}: {e}")
        
        self.root.after(0, lambda: self.status_label.config(text="Conversion complete!"))
        self.root.after(0, lambda: self.convert_btn.config(state='normal'))
        self.root.after(0, lambda: self.progress.config(value=0))

if __name__ == "__main__":
    root = tk.Tk()
    app = RawConverter(root)
    root.mainloop()