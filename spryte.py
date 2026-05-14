#!/usr/bin/env python3
"""Spryte — minimal 3-color (black / white / transparent) 20×20 pixel-art editor."""

import tkinter as tk
from tkinter import filedialog, messagebox
from collections import deque
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
BTN_FONT          = ("Helvetica", 10)
BTN_FONT_SELECTED = ("Helvetica", 10, "bold")
SWATCH_SZ  = 36

TOOLS = ["pencil", "rect", "move", "fill"]
TOOL_LABELS = {"pencil": "Pencil (Q)", "rect": "Rect (R)", "move": "Move (M)", "fill": "Fill (F)"}


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


def flood_fill(pixels: list[list[tuple]], col: int, row: int, fill_color: tuple):
    target = pixels[row][col]
    if target == fill_color:
        return
    q = deque([(col, row)])
    visited = set()
    while q:
        c, r = q.popleft()
        if (c, r) in visited or not (0 <= c < GRID and 0 <= r < GRID):
            continue
        if pixels[r][c] != target:
            continue
        visited.add((c, r))
        pixels[r][c] = fill_color
        q.extend([(c+1, r), (c-1, r), (c, r+1), (c, r-1)])


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
        self._cell: int = 28
        self._resize_job = None

        # tool state
        self._tool: str = "pencil"
        self._painting: bool = False
        self._history: list[list[list[tuple]]] = []

        # rect tool
        self._rect_start: tuple | None = None
        self._rect_end: tuple | None = None

        # move tool
        self._move_phase: str = "select"   # "select" | "floating"
        self._move_sel_start: tuple | None = None
        self._move_sel_end: tuple | None = None
        self._move_rect: tuple | None = None    # (c0, r0, c1, r1) normalized
        self._move_lifted: list | None = None   # 2D pixel array of lifted region
        self._move_offset: tuple = (0, 0)       # (dc, dr) applied to lifted pixels
        self._move_grab: tuple | None = None    # grid cell where drag started
        self._move_grab_offset: tuple = (0, 0) # offset at grab time

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

        self.canvas.bind("<Button-1>",        self._on_lmb_down)
        self.canvas.bind("<B1-Motion>",       self._on_lmb_drag)
        self.canvas.bind("<ButtonRelease-1>", self._on_lmb_up)
        self.canvas.bind("<Button-3>",        self._on_rmb)
        self.canvas.bind("<B3-Motion>",       self._on_rmb)
        self.canvas.bind("<Configure>",       self._on_canvas_resize)

        self.root.bind("1", lambda e: self._select_color(0))
        self.root.bind("2", lambda e: self._select_color(1))
        self.root.bind("3", lambda e: self._select_color(2))
        self.root.bind("q", lambda e: self._select_tool("pencil"))
        self.root.bind("r", lambda e: self._select_tool("rect"))
        self.root.bind("m", lambda e: self._select_tool("move"))
        self.root.bind("f", lambda e: self._select_tool("fill"))
        self.root.bind("<Escape>", lambda e: self._cancel_tool())
        self.root.bind("u", lambda e: self._undo())

    def _build_bottom_panel(self):
        panel = tk.Frame(self.root, bg=PANEL, pady=6)
        panel.pack(side=tk.BOTTOM, fill=tk.X)

        # Color swatches
        swatch_frame = tk.Frame(panel, bg=PANEL)
        swatch_frame.pack(side=tk.LEFT, padx=(8, 8))

        self._swatches: list[tk.Canvas] = []
        for i, (label, hex_c) in enumerate(zip(
            ["Trans.", "Black", "White"],
            [None, "#000000", "#ffffff"],
        )):
            self._build_swatch(swatch_frame, i, label, hex_c)
        self._refresh_swatches()

        # Tool buttons
        tool_frame = tk.Frame(panel, bg=PANEL)
        tool_frame.pack(side=tk.LEFT, padx=(0, 16))

        self._tool_buttons: dict[str, tk.Button] = {}
        for tool in TOOLS:
            b = tk.Button(tool_frame, text=TOOL_LABELS[tool],
                          command=lambda t=tool: self._select_tool(t),
                          bg=BTN_BG, fg=BTN_FG, activebackground=BTN_ACTIVE,
                          relief=tk.FLAT, borderwidth=0, highlightthickness=0,
                          padx=8, pady=4, anchor="w", font=BTN_FONT)
            b.pack(side=tk.TOP, fill=tk.X, pady=1)
            self._tool_buttons[tool] = b
        self._refresh_tool_buttons()

        # Sprite list
        list_frame = tk.Frame(panel, bg=PANEL)
        list_frame.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)

        tk.Label(list_frame, text="Sprites:", bg=PANEL, fg=FG,
                 font=("Helvetica", 10, "bold")).pack(side=tk.LEFT, padx=(0, 4))

        scrollbar = tk.Scrollbar(list_frame, orient=tk.HORIZONTAL, bg=PANEL,
                                 troughcolor=BG, width=8)
        self.sprite_list = tk.Listbox(
            list_frame,
            xscrollcommand=scrollbar.set,
            bg=BG, fg=FG, selectbackground=HIGHLIGHT,
            selectforeground="#ffffff",
            borderwidth=0, highlightthickness=0,
            font=("Helvetica", 10),
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
                             relief=tk.FLAT, borderwidth=0, highlightthickness=0,
                             padx=10, pady=5, font=BTN_FONT)

        btn("New",     self._cmd_new).pack(side=tk.TOP, fill=tk.X, pady=1)
        btn("Save",    self._cmd_save).pack(side=tk.TOP, fill=tk.X, pady=1)
        btn("Save As", self._cmd_save_as).pack(side=tk.TOP, fill=tk.X, pady=1)

        self.filename_label = tk.Label(panel, text="", bg=PANEL, fg=FG,
                                       font=("Helvetica", 10))
        self.filename_label.pack(side=tk.RIGHT, padx=(0, 12))

    def _build_swatch(self, parent, index: int, label: str, hex_color: str | None):
        container = tk.Frame(parent, bg=PANEL, cursor="hand2")
        container.pack(side=tk.LEFT, padx=4)

        if hex_color is None:
            sw = tk.Canvas(container, width=SWATCH_SZ, height=SWATCH_SZ,
                           highlightthickness=2, bd=0, highlightbackground=PANEL)
            h = SWATCH_SZ // 2
            sw.create_rectangle(0, 0, h,         h,         fill=CHECK_A, outline="")
            sw.create_rectangle(h, 0, SWATCH_SZ,  h,         fill=CHECK_B, outline="")
            sw.create_rectangle(0, h, h,          SWATCH_SZ, fill=CHECK_B, outline="")
            sw.create_rectangle(h, h, SWATCH_SZ,  SWATCH_SZ, fill=CHECK_A, outline="")
        else:
            sw = tk.Canvas(container, width=SWATCH_SZ, height=SWATCH_SZ,
                           bg=hex_color, highlightthickness=2, bd=0,
                           highlightbackground=PANEL)

        sw.pack()
        sw.bind("<Button-1>", lambda e, i=index: self._select_color(i))

        tk.Label(container, text=label, bg=PANEL, fg=FG,
                 font=("Helvetica", 9)).pack()
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
        self._draw_overlay()

    # ------------------------------------------------------------------
    # Undo
    # ------------------------------------------------------------------

    def _snapshot(self):
        self._history.append([row[:] for row in self.pixels])

    def _undo(self):
        if not self._history:
            return
        self._cancel_tool()
        self.pixels = self._history.pop()
        self.dirty = True
        self._draw_canvas()
        self._draw_overlay()

    # ------------------------------------------------------------------
    # Drawing
    # ------------------------------------------------------------------

    def _draw_canvas(self):
        self.canvas.delete("pixel")
        c = self._cell
        for row in range(GRID):
            for col in range(GRID):
                x0, y0 = col * c, row * c
                x1, y1 = x0 + c, y0 + c
                check = CHECK_A if (col + row) % 2 == 0 else CHECK_B
                self.canvas.create_rectangle(x0, y0, x1, y1, fill=check, outline="", tags="pixel")
                color = self.pixels[row][col]
                if color[3] > 0:
                    self.canvas.create_rectangle(x0, y0, x1, y1,
                                                 fill=pixel_color_hex(color), outline="", tags="pixel")

    def _redraw_cell(self, col: int, row: int):
        c = self._cell
        x0, y0 = col * c, row * c
        x1, y1 = x0 + c, y0 + c
        tag = f"cell_{col}_{row}"
        self.canvas.delete(tag)

        check = CHECK_A if (col + row) % 2 == 0 else CHECK_B
        self.canvas.create_rectangle(x0, y0, x1, y1, fill=check, outline="", tags=("pixel", tag))

        color = self.pixels[row][col]
        if color[3] > 0:
            self.canvas.create_rectangle(x0, y0, x1, y1,
                                         fill=pixel_color_hex(color), outline="", tags=("pixel", tag))

    def _draw_overlay(self):
        self.canvas.delete("overlay")
        c = self._cell

        if self._tool == "rect" and self._rect_start and self._rect_end:
            c0, r0 = self._rect_start
            c1, r1 = self._rect_end
            lc, rc = min(c0, c1), max(c0, c1)
            tr, br = min(r0, r1), max(r0, r1)
            border = set()
            for col in range(lc, rc + 1):
                border.add((col, tr))
                border.add((col, br))
            for row in range(tr, br + 1):
                border.add((lc, row))
                border.add((rc, row))
            hex_c = pixel_color_hex(self.current_color)
            for col, row in border:
                x0, y0 = col * c, row * c
                x1, y1 = x0 + c, y0 + c
                fill = hex_c if hex_c else (CHECK_A if (col + row) % 2 == 0 else CHECK_B)
                self.canvas.create_rectangle(x0, y0, x1, y1, fill=fill, outline="", tags="overlay")

        elif self._tool == "move":
            if self._move_phase == "select" and self._move_sel_start and self._move_sel_end:
                c0, r0 = self._move_sel_start
                c1, r1 = self._move_sel_end
                x0 = min(c0, c1) * c
                y0 = min(r0, r1) * c
                x1 = (max(c0, c1) + 1) * c
                y1 = (max(r0, r1) + 1) * c
                self.canvas.create_rectangle(x0, y0, x1, y1,
                                             outline="white", dash=(4, 4), width=1, tags="overlay")

            elif self._move_phase == "floating" and self._move_lifted and self._move_rect:
                mc0, mr0, mc1, mr1 = self._move_rect
                dc, dr = self._move_offset
                for row in range(mr0, mr1 + 1):
                    for col in range(mc0, mc1 + 1):
                        px = self._move_lifted[row - mr0][col - mc0]
                        nc, nr = col + dc, row + dr
                        if not (0 <= nc < GRID and 0 <= nr < GRID):
                            continue
                        x0, y0 = nc * c, nr * c
                        check = CHECK_A if (nc + nr) % 2 == 0 else CHECK_B
                        self.canvas.create_rectangle(x0, y0, x0 + c, y0 + c,
                                                     fill=check, outline="", tags="overlay")
                        if px[3] > 0:
                            self.canvas.create_rectangle(x0, y0, x0 + c, y0 + c,
                                                         fill=pixel_color_hex(px),
                                                         outline="", tags="overlay")
                # selection border
                bx0 = (mc0 + dc) * c
                by0 = (mr0 + dr) * c
                bx1 = (mc1 + dc + 1) * c
                by1 = (mr1 + dr + 1) * c
                self.canvas.create_rectangle(bx0, by0, bx1, by1,
                                             outline="white", dash=(4, 4), width=1, tags="overlay")

    # ------------------------------------------------------------------
    # Tool selection
    # ------------------------------------------------------------------

    def _select_tool(self, tool: str):
        self._cancel_tool()
        self._tool = tool
        self._refresh_tool_buttons()

    def _refresh_tool_buttons(self):
        for tool, btn in self._tool_buttons.items():
            selected = tool == self._tool
            btn.config(
                bg=HIGHLIGHT if selected else BTN_BG,
                font=BTN_FONT_SELECTED if selected else BTN_FONT,
            )

    def _cancel_tool(self):
        if self._tool == "move" and self._move_phase == "floating":
            self._move_restore()
        self._rect_start = self._rect_end = None
        self._move_phase = "select"
        self._move_sel_start = self._move_sel_end = None
        self._move_rect = self._move_lifted = None
        self._move_offset = (0, 0)
        self._draw_overlay()

    # ------------------------------------------------------------------
    # Mouse event dispatch
    # ------------------------------------------------------------------

    def _on_lmb_down(self, event):
        self._painting = True
        pos = self._canvas_to_grid(event.x, event.y)
        if self._tool == "pencil":
            if pos:
                self._snapshot()
                self._paint_pixel(pos, self.current_color)
        elif self._tool == "rect":
            self._rect_start = pos
            self._rect_end = pos
            self._draw_overlay()
        elif self._tool == "move":
            self._on_move_down(pos)
        elif self._tool == "fill":
            if pos:
                self._snapshot()
                col, row = pos
                flood_fill(self.pixels, col, row, self.current_color)
                self.dirty = True
                self._draw_canvas()
                self._draw_overlay()

    def _on_lmb_drag(self, event):
        if not self._painting:
            return
        pos = self._canvas_to_grid(event.x, event.y)
        if self._tool == "pencil":
            if pos:
                self._paint_pixel(pos, self.current_color)
        elif self._tool == "rect":
            if pos:
                self._rect_end = pos
                self._draw_overlay()
        elif self._tool == "move":
            self._on_move_drag(pos)

    def _on_lmb_up(self, event):
        self._painting = False
        pos = self._canvas_to_grid(event.x, event.y)
        if self._tool == "rect":
            self._commit_rect()
        elif self._tool == "move":
            self._on_move_up(pos)

    def _on_rmb(self, event):
        pos = self._canvas_to_grid(event.x, event.y)
        if self._tool == "pencil" and pos:
            self._paint_pixel(pos, TRANSPARENT)

    # ------------------------------------------------------------------
    # Pencil
    # ------------------------------------------------------------------

    def _paint_pixel(self, pos: tuple, color: tuple):
        col, row = pos
        if self.pixels[row][col] == color:
            return
        self.pixels[row][col] = color
        self.dirty = True
        self._redraw_cell(col, row)

    # ------------------------------------------------------------------
    # Rect tool
    # ------------------------------------------------------------------

    def _commit_rect(self):
        if not self._rect_start or not self._rect_end:
            return
        self._snapshot()
        c0, r0 = self._rect_start
        c1, r1 = self._rect_end
        lc, rc = min(c0, c1), max(c0, c1)
        tr, br = min(r0, r1), max(r0, r1)
        for col in range(lc, rc + 1):
            self.pixels[tr][col] = self.current_color
            self.pixels[br][col] = self.current_color
        for row in range(tr, br + 1):
            self.pixels[row][lc] = self.current_color
            self.pixels[row][rc] = self.current_color
        self.dirty = True
        self._rect_start = self._rect_end = None
        self._draw_canvas()
        self._draw_overlay()

    # ------------------------------------------------------------------
    # Move tool
    # ------------------------------------------------------------------

    def _on_move_down(self, pos: tuple | None):
        if self._move_phase == "select":
            self._move_sel_start = pos
            self._move_sel_end = pos
            self._draw_overlay()
        elif self._move_phase == "floating":
            # begin drag of floating selection
            self._move_grab = pos
            self._move_grab_offset = self._move_offset

    def _on_move_drag(self, pos: tuple | None):
        if not pos:
            return
        if self._move_phase == "select":
            self._move_sel_end = pos
            self._draw_overlay()
        elif self._move_phase == "floating" and self._move_grab:
            gc, gr = self._move_grab
            pc, pr = pos
            go_c, go_r = self._move_grab_offset
            self._move_offset = (go_c + pc - gc, go_r + pr - gr)
            self._draw_canvas()
            self._draw_overlay()

    def _on_move_up(self, pos: tuple | None):
        if self._move_phase == "select":
            if not self._move_sel_start or not self._move_sel_end:
                return
            c0, r0 = self._move_sel_start
            c1, r1 = self._move_sel_end
            nc0, nc1 = min(c0, c1), max(c0, c1)
            nr0, nr1 = min(r0, r1), max(r0, r1)
            self._move_rect = (nc0, nr0, nc1, nr1)
            # lift pixels
            self._move_lifted = [
                [self.pixels[row][col] for col in range(nc0, nc1 + 1)]
                for row in range(nr0, nr1 + 1)
            ]
            for row in range(nr0, nr1 + 1):
                for col in range(nc0, nc1 + 1):
                    self.pixels[row][col] = TRANSPARENT
            self._move_offset = (0, 0)
            self._move_grab = None
            self._move_phase = "floating"
            self._draw_canvas()
            self._draw_overlay()

        elif self._move_phase == "floating":
            self._move_grab = None
            self._move_commit()

    def _move_restore(self):
        """Put lifted pixels back at original position."""
        if not self._move_rect or not self._move_lifted:
            return
        mc0, mr0, mc1, mr1 = self._move_rect
        for row in range(mr0, mr1 + 1):
            for col in range(mc0, mc1 + 1):
                self.pixels[row][col] = self._move_lifted[row - mr0][col - mc0]
        self.dirty = True
        self._move_phase = "select"
        self._move_rect = self._move_lifted = None
        self._move_sel_start = self._move_sel_end = None
        self._move_offset = (0, 0)
        self._draw_canvas()
        self._draw_overlay()

    def _move_commit(self):
        """Paste lifted pixels at current offset and end move."""
        if not self._move_rect or not self._move_lifted:
            return
        self._snapshot()
        mc0, mr0, mc1, mr1 = self._move_rect
        dc, dr = self._move_offset
        for row in range(mr0, mr1 + 1):
            for col in range(mc0, mc1 + 1):
                nc, nr = col + dc, row + dr
                if 0 <= nc < GRID and 0 <= nr < GRID:
                    self.pixels[nr][nc] = self._move_lifted[row - mr0][col - mc0]
        self.dirty = True
        self._move_phase = "select"
        self._move_rect = self._move_lifted = None
        self._move_sel_start = self._move_sel_end = None
        self._move_offset = (0, 0)
        self._draw_canvas()
        self._draw_overlay()

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
    # Helpers
    # ------------------------------------------------------------------

    def _canvas_to_grid(self, cx: int, cy: int) -> tuple[int, int] | None:
        c = self._cell
        col, row = cx // c, cy // c
        return (col, row) if 0 <= col < GRID and 0 <= row < GRID else None

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
        self._cancel_tool()
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
        self._cancel_tool()
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
