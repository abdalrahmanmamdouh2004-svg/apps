#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
cmf_wp2_app::app_icon!();

use cmf_wp2_app::{Api, BUTTON, CROWN, TOUCH_DOWN};

#[cfg(not(test))]
cmf_wp2_app::entry!(main);
#[cfg(not(test))]
cmf_wp2_app::app_features!(cmf_wp2_app::WIDGET | cmf_wp2_app::PAGE);

cmf_wp2_app::app_features!(cmf_wp2_app::WIDGET | cmf_wp2_app::PAGE);

const BG_COLOR: u16 = 0x0000;
const GRID_DOT_COLOR: u16 = 0x52AA;
const BEZEL_DOT_COLOR: u16 = 0x2945;
const WHITE: u16 = 0xFFFF;
const CMF_ORANGE: u16 = 0xFA60;
const WIN_COLOR: u16 = 0xF800;

const BOARD_SIZE: i16 = 276;
const CELL_SIZE: i16 = BOARD_SIZE / 3;

const MATRIX_X: [u8; 25] = [
    1, 0, 0, 0, 1,
    0, 1, 0, 1, 0,
    0, 0, 1, 0, 0,
    0, 1, 0, 1, 0,
    1, 0, 0, 0, 1,
];

const MATRIX_O: [u8; 25] = [
    0, 1, 1, 1, 0,
    1, 0, 0, 0, 1,
    1, 0, 0, 0, 1,
    1, 0, 0, 0, 1,
    0, 1, 1, 1, 0,
];

const WIN_LINES: [usize; 24] = [
    0, 1, 2,
    3, 4, 5,
    6, 7, 8,
    0, 3, 6,
    1, 4, 7,
    2, 5, 8,
    0, 4, 8,
    2, 4, 6,
];

fn draw_dot(api: &Api, cx: i16, cy: i16, r: i16, color: u16) {
    if r <= 0 {
        return;
    }
    if r == 1 {
        api.rect(cx, cy, 2, 2, color);
        return;
    }
    if r == 2 {
        api.rect(cx - 1, cy - 2, 3, 5, color);
        api.rect(cx - 2, cy - 1, 5, 3, color);
        return;
    }
    if r == 3 {
        api.rect(cx - 1, cy - 3, 3, 7, color);
        api.rect(cx - 2, cy - 2, 5, 5, color);
        api.rect(cx - 3, cy - 1, 7, 3, color);
        return;
    }
    api.rect(cx - 1, cy - 4, 3, 9, color);
    api.rect(cx - 3, cy - 3, 7, 7, color);
    api.rect(cx - 4, cy - 1, 9, 3, color);
}

fn draw_dot_grid(api: &Api, offset_x: i16, offset_y: i16, cx: i16, cy: i16, intro_step: i16) {
    let dots_count: i16 = 25;
    let step = BOARD_SIZE / (dots_count - 1);

    for line_idx in 1..=2 {
        let fixed_x = offset_x + line_idx * CELL_SIZE;
        let fixed_y = offset_y + line_idx * CELL_SIZE;

        for i in 0..dots_count {
            let var_pos = i * step;

            let vx = fixed_x;
            let vy = offset_y + var_pos;
            let dist_v = (vx - cx).abs() + (vy - cy).abs();
            let r_v = (intro_step - dist_v / 14).clamp(0, 3);
            draw_dot(api, vx, vy, r_v, GRID_DOT_COLOR);

            let hx = offset_x + var_pos;
            let hy = fixed_y;
            let dist_h = (hx - cx).abs() + (hy - cy).abs();
            let r_h = (intro_step - dist_h / 14).clamp(0, 3);
            draw_dot(api, hx, hy, r_h, GRID_DOT_COLOR);
        }
    }
}

fn draw_dot_symbol(
    api: &Api,
    cell_idx: usize,
    player: u8,
    offset_x: i16,
    offset_y: i16,
    anim_r: i16,
    highlight: bool,
) {
    let col = (cell_idx % 3) as i16;
    let row = (cell_idx / 3) as i16;

    let cell_cx = offset_x + col * CELL_SIZE + CELL_SIZE / 2;
    let cell_cy = offset_y + row * CELL_SIZE + CELL_SIZE / 2;

    let matrix = if player == 1 { &MATRIX_X } else { &MATRIX_O };
    let color = if highlight {
        WIN_COLOR
    } else if player == 1 {
        CMF_ORANGE
    } else {
        WHITE
    };

    let dot_gap: i16 = 12;
    let start_x = cell_cx - 2 * dot_gap;
    let start_y = cell_cy - 2 * dot_gap;

    for r in 0..5 {
        for c in 0..5 {
            if matrix[r * 5 + c] == 1 {
                let dist_center = ((r as i16 - 2).abs() + (c as i16 - 2).abs()) / 2;
                let radius = (anim_r - dist_center).clamp(0, 4);
                let px = start_x + (c as i16) * dot_gap;
                let py = start_y + (r as i16) * dot_gap;
                draw_dot(api, px, py, radius, color);
            }
        }
    }
}

fn check_winner(board: &[u8; 9]) -> (u8, Option<usize>) {
    for line_idx in 0..8 {
        let base = line_idx * 3;
        let a = board[WIN_LINES[base]];
        if a != 0 && a == board[WIN_LINES[base + 1]] && a == board[WIN_LINES[base + 2]] {
            return (a, Some(line_idx));
        }
    }
    let mut full = true;
    for &cell in board.iter() {
        if cell == 0 {
            full = false;
            break;
        }
    }
    if full {
        (3, None)
    } else {
        (0, None)
    }
}

fn minimax(board: &mut [u8; 9], depth: i32, is_max: bool) -> i32 {
    let (status, _) = check_winner(board);
    if status == 2 {
        return 10 - depth;
    }
    if status == 1 {
        return depth - 10;
    }
    if status == 3 {
        return 0;
    }

    if is_max {
        let mut best = -100;
        for i in 0..9 {
            if board[i] == 0 {
                board[i] = 2;
                let score = minimax(board, depth + 1, false);
                board[i] = 0;
                if score > best {
                    best = score;
                }
            }
        }
        best
    } else {
        let mut best = 100;
        for i in 0..9 {
            if board[i] == 0 {
                board[i] = 1;
                let score = minimax(board, depth + 1, true);
                board[i] = 0;
                if score < best {
                    best = score;
                }
            }
        }
        best
    }
}

fn choose_bot_move(board: &mut [u8; 9], seed: u32) -> Option<usize> {
    let mut empty = [0usize; 9];
    let mut count = 0usize;
    for i in 0..9 {
        if board[i] == 0 {
            empty[count] = i;
            count += 1;
        }
    }
    if count == 0 {
        return None;
    }

    if (seed % 5) == 0 {
        return Some(empty[(seed as usize / 7) % count]);
    }

    let mut best_score = -100;
    let mut best_move = empty[0];
    for idx in 0..count {
        let pos = empty[idx];
        board[pos] = 2;
        let score = minimax(board, 0, false);
        board[pos] = 0;
        if score > best_score {
            best_score = score;
            best_move = pos;
        }
    }
    Some(best_move)
}

fn main(api: &'static Api) -> i32 {
    let width = api.width as i16;
    let height = api.height as i16;
    let cx = width / 2;
    let cy = height / 2;

    let offset_x = (width - BOARD_SIZE) / 2;
    let offset_y = (height - BOARD_SIZE) / 2;

    let mut board = [0u8; 9];
    let mut cell_anim = [0i16; 9];

    let mut player_wins: u8 = 0;
    let mut bot_wins: u8 = 0;

    let mut state: u8 = 0;
    let mut winner: u8 = 0;
    let mut win_line_idx: Option<usize> = None;

    let mut intro_step: i16 = 0;
    let mut last_tick = api.now();
    let mut bot_wait_start = 0u32;

    loop {
        let now = api.now();

        if now.wrapping_sub(last_tick) >= 25 {
            last_tick = now;

            if state == 0 {
                intro_step += 2;
                if intro_step >= 26 {
                    state = 1;
                }
            }

            for i in 0..9 {
                if board[i] != 0 && cell_anim[i] < 6 {
                    cell_anim[i] += 1;
                }
            }

            if state == 2 && now.wrapping_sub(bot_wait_start) >= 350 {
                if let Some(m) = choose_bot_move(&mut board, now) {
                    board[m] = 2;
                    cell_anim[m] = 1;
                }
                let (res, line) = check_winner(&board);
                if res != 0 {
                    winner = res;
                    win_line_idx = line;
                    state = 3;
                    if res == 1 {
                        player_wins = player_wins.saturating_add(1);
                    } else if res == 2 {
                        bot_wins = bot_wins.saturating_add(1);
                    }
                } else {
                    state = 1;
                }
            }
        }

        api.rect(0, 0, api.width, api.height, BG_COLOR);

        draw_dot(api, cx, 24, 2, CMF_ORANGE);
        draw_dot(api, cx, height - 24, 2, BEZEL_DOT_COLOR);

        api.text(cx - 56, 42, "DOT X/O", WHITE);

        let mut score_str = [b'X', b':', b'0', b' ', b' ', b'O', b':', b'0'];
        score_str[2] = b'0' + (player_wins % 10);
        score_str[7] = b'0' + (bot_wins % 10);
        let score_label = core::str::from_utf8(&score_str).unwrap_or("X:0  O:0");
        api.text(cx - 32, 62, score_label, CMF_ORANGE);

        draw_dot_grid(api, offset_x, offset_y, cx, cy, intro_step);

        for i in 0..9 {
            if board[i] != 0 {
                let is_win_cell = if let Some(l_idx) = win_line_idx {
                    let base = l_idx * 3;
                    WIN_LINES[base] == i || WIN_LINES[base + 1] == i || WIN_LINES[base + 2] == i
                } else {
                    false
                };
                draw_dot_symbol(
                    api,
                    i,
                    board[i],
                    offset_x,
                    offset_y,
                    cell_anim[i],
                    is_win_cell,
                );
            }
        }

        match state {
            0 => api.text(cx - 44, height - 62, "LOADING...", GRID_DOT_COLOR),
            1 => api.text(cx - 48, height - 62, "YOUR TURN", WHITE),
            2 => api.text(cx - 44, height - 62, "BOT TURN..", CMF_ORANGE),
            _ => {
                if winner == 1 {
                    api.text(cx - 40, height - 68, "YOU WIN!", CMF_ORANGE);
                } else if winner == 2 {
                    api.text(cx - 40, height - 68, "BOT WINS", WHITE);
                } else {
                    api.text(cx - 24, height - 68, "DRAW!", GRID_DOT_COLOR);
                }
                api.text(cx - 56, height - 48, "TAP TO RETRY", GRID_DOT_COLOR);
            }
        }

        api.show();

        if let Some(event) = api.event(20) {
            match event.kind {
                BUTTON => return 0,

                CROWN => {
                    if state == 3 {
                        board = [0u8; 9];
                        cell_anim = [0i16; 9];
                        winner = 0;
                        win_line_idx = None;
                        intro_step = 0;
                        state = 0;
                    }
                }

                TOUCH_DOWN => {
                    if state == 0 {
                        intro_step = 26;
          state = 1;
} else if state == 3 {
board = [0u8; 9];
cell_anim = [0i16; 9];
winner = 0;
win_line_idx = None;
intro_step = 0;
state = 0;
} else if state == 1 {
let tx = event.x;
let ty = event.y;
if tx >= offset_x
&& tx < offset_x + BOARD_SIZE
&& ty >= offset_y
&& ty < offset_y + BOARD_SIZE
{
let col = ((tx - offset_x) / CELL_SIZE) as usize;
let row = ((ty - offset_y) / CELL_SIZE) as usize;
let idx = row * 3 + col;
if idx < 9 && board[idx] == 0 {
board[idx] = 1;
cell_anim[idx] = 1;
let (res, line) = check_winner(&board);
if res != 0 {
winner = res;
win_line_idx = line;
state = 3;
if res == 1 {
player_wins = player_wins.saturating_add(1);
}
} else {
state = 2;
bot_wait_start = api.now();
}
}
}
}
}
_ => {}
}
}
}
}
