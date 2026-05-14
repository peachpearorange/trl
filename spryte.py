#!/usr/bin/env python3
"""Spryte — minimal 3-color (black / white / transparent) 20×20 pixel-art editor."""

import tkinter as tk
from tkinter import filedialog, messagebox
from pathlib import Path
from PIL import Image

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

SCRIPT_DIR = Path(__file__).parent
ASSETS_DIR = SCRIPT_DIR / "assets" / "textures" / "space_qud"

GRID = 20

TRANSPARENT = (0, 0, 0, 0)
BLACK       = (0, 0, 0, 255)
WHITE       = (255, 255, 255, 255)
PALETTE     = [TRANSPARENT, BLACK, WHITE]

CHECK_A = "#888888"
CHECK_B = "#aaaaaa"

BG         = "#2b2b2b"
PANEL      = "#3c3f41"
FG         = "#bbbbbb"
HIGHLIGHT  = "#4e9fea"
BTN_BG     = "#4c5052"
BTN_FG     = "#dddddd"
BTN_ACTIVE = "#5c6366"
SWATCH_SZ  = 36


# ---------------------------------------------------------------------------
# Model
# ---------------------------------------------------------------------------

def blank_canvas() -> list[list[tuple]]:
    return [[TRANSPARENT] * GRID for _ in range(GRID)]


def load_image(path: Path) -> list[list[tuple]]:
    img = Image.open(path).convert("RGBA").resize((GRID, GRID), Image.NEAREST)
    pixels = blank_canvas()
    for y in range(GRID):
        for x in range(GRID):
            r, g, b, a = img.getpixel((x, y))
            if a < 128:
                pixels[y][x] = TRANSPARENT
            else:
                lum = 0.299 * r + 0.587 * g + 0.114 * b
                pixels[y][x] = WHITE if lum >= 128 else BLACK
    return pixels


def save_image(path: Path, pixels: list[list[tuple]]):
    img = Image.new("RGBA", (GRID, GRID), (0, 0, 0, 0))
    for y in range(GRID):
        for x in range(GRID):
            img.putpixel((x, y), pixels[y][x])
    img.save(path, "PNG")


def pixel_color_hex(color: tuple) -> str | None:
    if color[3] == 0:
        return None
    return "#{:02x}{:02x}{:02x}".format(color[0], color[1], color[2])


# ---------------------------------------------------------------------------
# Main application
# ---------------------------------------------------------------------------

class Spryte:
    def __init__(self, root: tk.Tk):
        self.root = root
        self.root.title("Spryte")
        self.root.configure(bg=BG)

        self.pixels: list[list[tuple]] = blank_canvas()
        self.current_color: tuple = BLACK
        self.current_file: Path | None = None
        self.dirty: bool = False
        self._painting: bool = False
        self._cell: int = 28
        self._resize_job = None

        self._build_ui()
        self._update_title()

    # ------------------------------------------------------------------
    # UI construction
    # ------------------------------------------------------------------

    def _build_ui(self):
        self._build_canvas()
        self._build_bottom_panel()

    def _build_canvas(self):
        self.canvas = tk.Canvas(self.root, bg=BG, highlightthickness=0, cursor="crosshair")
        self.canvas.pack(side=tk.TOP, fill=tk.BOTH, expand=True)

        self.canvas.bind("<Button-1>",        self._on_paint_start)
        self.canvas.bind("<B1-Motion>",       self._on_paint_drag)
        self.canvas.bind("<ButtonRelease-1>", self._on_paint_end)
        self.canvas.bind("<Button-3>",        self._on_erase)
        self.canvas.bind("<B3-Motion>",       self._on_erase)
        self.canvas.bind("<Configure>",       self._on_canvas_resize)

    def _build_bottom_panel(self):
        panel = tk.Frame(self.root, bg=PANEL, pady=6)
        panel.pack(side=tk.BOTTOM, fill=tk.X)

        # Color swatches
        swatch_frame = tk.Frame(panel, bg=PANEL)
        swatch_frame.pack(side=tk.LEFT, padx=(8, 16))

        self._swatches: list[tk.Canvas] = []
        for i, (label, hex_c) in enumerate(zip(
            ["Trans.", "Black", "White"],
            [None, "#000000", "#ffffff"],
        )):
            self._build_swatch(swatch_frame, i, label, hex_c)

        self._refresh_swatches()

        # Sprite list
        list_frame = tk.Frame(panel, bg=PANEL)
        list_frame.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)

        tk.Label(list_frame, text="Sprites:", bg=PANEL, fg=FG,
                 font=("Helvetica", 8, "bold")).pack(side=tk.LEFT, padx=(0, 4))

        scrollbar = tk.Scrollbar(list_frame, orient=tk.HORIZONTAL, bg=PANEL,
                                 troughcolor=BG, width=8)
        self.sprite_list = tk.Listbox(
            list_frame,
            xscrollcommand=scrollbar.set,
            bg=BG, fg=FG, selectbackground=HIGHLIGHT,
            selectforeground="#ffffff",
            borderwidth=0, highlightthickness=0,
            font=("Helvetica", 8),
            activestyle="none",
            height=3,
        )
        scrollbar.config(command=self.sprite_list.xview)
        self.sprite_list.pack(side=tk.TOP, fill=tk.BOTH, expand=True)
        scrollbar.pack(side=tk.TOP, fill=tk.X)

        self.sprite_list.bind("<Double-Button-1>", self._on_sprite_open)
        self.sprite_list.bind("<Return>",          self._on_sprite_open)

        self._refresh_sprite_list()

        # File buttons + filename
        btn_frame = tk.Frame(panel, bg=PANEL)
        btn_frame.pack(side=tk.RIGHT, padx=8)

        def btn(text, cmd):
            return tk.Button(btn_frame, text=text, command=cmd,
                             bg=BTN_BG, fg=BTN_FG,
                             activebackground=BTN_ACTIVE,
                             relief=tk.FLAT, padx=8, pady=3)

        btn("New",     self._cmd_new).pack(side=tk.TOP, fill=tk.X, pady=1)
        btn("Save",    self._cmd_save).pack(side=tk.TOP, fill=tk.X, pady=1)
        btn("Save As", self._cmd_save_as).pack(side=tk.TOP, fill=tk.X, pady=1)

        self.filename_label = tk.Label(panel, text="", bg=PANEL, fg=FG,
                                       font=("Helvetica", 8))
        self.filename_label.pack(side=tk.RIGHT, padx=(0, 12))

    def _build_swatch(self, parent, index: int, label: str, hex_color: str | None):
        container = tk.Frame(parent, bg=PANEL, cursor="hand2")
        container.pack(side=tk.LEFT, padx=4)

        if hex_color is None:
            sw = tk.Canvas(container, width=SWATCH_SZ, height=SWATCH_SZ,
                           highlightthickness=2, bd=0, highlightbackground=PANEL)
            h = SWATCH_SZ // 2
            sw.create_rectangle(0, 0, h,        h,         fill=CHECK_A, outline="")
            sw.create_rectangle(h, 0, SWATCH_SZ, h,         fill=CHECK_B, outline="")
            sw.create_rectangle(0, h, h,         SWATCH_SZ, fill=CHECK_B, outline="")
            sw.create_rectangle(h, h, SWATCH_SZ, SWATCH_SZ, fill=CHECK_A, outline="")
        else:
            sw = tk.Canvas(container, width=SWATCH_SZ, height=SWATCH_SZ,
                           bg=hex_color, highlightthickness=2, bd=0,
                           highlightbackground=PANEL)

        sw.pack()
        sw.bind("<Button-1>", lambda e, i=index: self._select_color(i))

        tk.Label(container, text=label, bg=PANEL, fg=FG,
                 font=("Helvetica", 7)).pack()
        container.bind("<Button-1>", lambda e, i=index: self._select_color(i))

        self._swatches.append(sw)

    # ------------------------------------------------------------------
    # Canvas resize
    # ------------------------------------------------------------------

    def _on_canvas_resize(self, event):
        if self._resize_job is not None:
            self.root.after_cancel(self._resize_job)
        self._resize_job = self.root.after(50, lambda: self._apply_resize(event.width, event.height))

    def _apply_resize(self, w: int, h: int):
        self._resize_job = None
        self._cell = max(1, min(w, h) // GRID)
        self._draw_canvas()

    # ------------------------------------------------------------------
    # Drawing
    # ------------------------------------------------------------------

    def _draw_canvas(self):
        self.canvas.delete("all")
        c = self._cell
        for row in range(GRID):
            for col in range(GRID):
                x0, y0 = col * c, row * c
                x1, y1 = x0 + c, y0 + c
                check = CHECK_A if (col + row) % 2 == 0 else CHECK_B
                self.canvas.create_rectangle(x0, y0, x1, y1, fill=check, outline="")
                color = self.pixels[row][col]
                if color[3] > 0:
                    self.canvas.create_rectangle(x0, y0, x1, y1,
                                                 fill=pixel_color_hex(color), outline="")

    def _redraw_cell(self, col: int, row: int):
        c = self._cell
        x0, y0 = col * c, row * c
        x1, y1 = x0 + c, y0 + c
        tag = f"cell_{col}_{row}"
        self.canvas.delete(tag)

        check = CHECK_A if (col + row) % 2 == 0 else CHECK_B
        self.canvas.create_rectangle(x0, y0, x1, y1, fill=check, outline="", tags=tag)

        color = self.pixels[row][col]
        if color[3] > 0:
            self.canvas.create_rectangle(x0, y0, x1, y1,
                                         fill=pixel_color_hex(color), outline="", tags=tag)

    # ------------------------------------------------------------------
    # Color palette
    # ------------------------------------------------------------------

    def _select_color(self, index: int):
        self.current_color = PALETTE[index]
        self._refresh_swatches()

    def _refresh_swatches(self):
        for i, sw in enumerate(self._swatches):
            sw.config(highlightbackground=HIGHLIGHT if PALETTE[i] == self.current_color else PANEL)

    # ------------------------------------------------------------------
    # Painting
    # ------------------------------------------------------------------

    def _canvas_to_grid(self, cx: int, cy: int) -> tuple[int, int] | None:
        c = self._cell
        col, row = cx // c, cy // c
        return (col, row) if 0 <= col < GRID and 0 <= row < GRID else None

    def _on_paint_start(self, event):
        self._painting = True
        self._paint_at(event.x, event.y, self.current_color)

    def _on_paint_drag(self, event):
        if self._painting:
            self._paint_at(event.x, event.y, self.current_color)

    def _on_paint_end(self, event):
        self._painting = False

    def _on_erase(self, event):
        self._paint_at(event.x, event.y, TRANSPARENT)

    def _paint_at(self, cx: int, cy: int, color: tuple):
        pos = self._canvas_to_grid(cx, cy)
        if pos is None:
            return
        col, row = pos
        if self.pixels[row][col] == color:
            return
        self.pixels[row][col] = color
        self.dirty = True
        self._redraw_cell(col, row)

    # ------------------------------------------------------------------
    # Sprite list
    # ------------------------------------------------------------------

    def _refresh_sprite_list(self):
        self.sprite_list.delete(0, tk.END)
        if ASSETS_DIR.exists():
            for p in sorted(ASSETS_DIR.glob("*.png"), key=lambda p: p.name.lower()):
                self.sprite_list.insert(tk.END, p.name)

    def _on_sprite_open(self, event=None):
        sel = self.sprite_list.curselection()
        if not sel:
            return
        path = ASSETS_DIR / self.sprite_list.get(sel[0])
        if not path.exists():
            messagebox.showerror("Error", f"File not found:\n{path}")
            return
        if self.dirty and not self._confirm_discard():
            return
        self._open(path)

    # ------------------------------------------------------------------
    # File commands
    # ------------------------------------------------------------------

    def _cmd_new(self):
        if self.dirty and not self._confirm_discard():
            return
        self.pixels = blank_canvas()
        self.current_file = None
        self.dirty = False
        self._draw_canvas()
        self._update_title()

    def _cmd_save(self):
        if self.current_file is None:
            self._cmd_save_as()
        else:
            self._save(self.current_file)

    def _cmd_save_as(self):
        initial_dir = str(ASSETS_DIR) if ASSETS_DIR.exists() else str(SCRIPT_DIR)
        path = filedialog.asksaveasfilename(
            defaultextension=".png",
            filetypes=[("PNG files", "*.png")],
            initialdir=initial_dir,
            title="Save Sprite As",
        )
        if path:
            self._save(Path(path))

    def _save(self, path: Path):
        path.parent.mkdir(parents=True, exist_ok=True)
        save_image(path, self.pixels)
        self.current_file = path
        self.dirty = False
        self._update_title()
        self._refresh_sprite_list()

    def _open(self, path: Path):
        self.pixels = load_image(path)
        self.current_file = path
        self.dirty = False
        self._draw_canvas()
        self._update_title()

    def _confirm_discard(self) -> bool:
        return messagebox.askyesno("Unsaved changes",
                                   "You have unsaved changes. Discard them?")

    def _update_title(self):
        name = self.current_file.name if self.current_file else "untitled"
        self.root.title(f"Spryte — {name}")
        self.filename_label.config(text=name)


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

def main():
    root = tk.Tk()
    Spryte(root)
    root.mainloop()


if __name__ == "__main__":
    main()
