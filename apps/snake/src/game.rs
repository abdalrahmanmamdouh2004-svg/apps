cmf_wp2_app::app_icon!();

use cmf_wp2_app::{Api, BUTTON, CROWN, TOUCH_DOWN};

cmf_wp2_app::entry!(main);
cmf_wp2_app::app_features!(cmf_wp2_app::WIDGET | cmf_wp2_app::PAGE);

const CELL: i16 = 12;
const MAX: usize = 900;

const BG_COLOR: u16 = 0x0000;
const GRID_COLOR: u16 = 0x4208;
const BUTTON_COLOR: u16 = 0x2104;
const BORDER_COLOR: u16 = 0x630C;
const WHITE: u16 = 0xFFFF;
const SNAKE_COLOR: u16 = 0x05A0;
const HEAD_COLOR: u16 = 0x07E0;
const FOOD_COLOR: u16 = 0xF800;

const SIDE_W: i16 = 82;
const SIDE_H: i16 = 96;
const TOP_W: i16 = 136;
const TOP_H: i16 = 74;
const GAP: i16 = 8;
const GRID_TOP: i16 = 92;
const BOTTOM_AREA: i16 = 58;

// Рисуем стрелку
// 0 — вверх, 1 — вправо, 2 — вниз, 3 — влево
fn draw_arrow(
    api: &Api,
    cx: i16,
    cy: i16,
    size: i16,
    direction: usize,
) {
    let head = size / 2;
    let half_head = head / 2;
    let shaft = size / 4;
    let half_shaft = shaft / 2;

    match direction {
        0 => {
            let top = cy - size / 2;

            for row in 0..head {
                let half = (row * half_head) / head.max(1);

                api.rect(
                    cx - half,
                    top + row,
                    (half * 2 + 2) as u16,
                    1,
                    WHITE,
                );
            }

            api.rect(
                cx - half_shaft,
                top + head - 1,
                shaft as u16,
                (size - head + 1) as u16,
                WHITE,
            );
        }

        1 => {
            let left = cx - size / 2;

            api.rect(
                left,
                cy - half_shaft,
                (size - head + 1) as u16,
                shaft as u16,
                WHITE,
            );

            for col in 0..head {
                let half = (col * half_head) / head.max(1);

                api.rect(
                    left + size - head + col,
                    cy - half,
                    1,
                    (half * 2 + 2) as u16,
                    WHITE,
                );
            }
        }

        2 => {
            let top = cy - size / 2;

            api.rect(
                cx - half_shaft,
                top,
                shaft as u16,
                (size - head + 1) as u16,
                WHITE,
            );

            for row in 0..head {
                let half =
                    ((head - row) * half_head) / head.max(1);

                api.rect(
                    cx - half,
                    top + size - head + row,
                    (half * 2 + 2) as u16,
                    1,
                    WHITE,
                );
            }
        }

        _ => {
            let left = cx - size / 2;

            for col in 0..head {
                let half =
                    ((head - col) * half_head) / head.max(1);

                api.rect(
                    left + col,
                    cy - half,
                    1,
                    (half * 2 + 2) as u16,
                    WHITE,
                );
            }

            api.rect(
                left + head - 1,
                cy - half_shaft,
                (size - head + 1) as u16,
                shaft as u16,
                WHITE,
            );
        }
    }
}

// Отдельная прямоугольная кнопка
fn draw_button(
    api: &Api,
    x: i16,
    y: i16,
    w: i16,
    h: i16,
    direction: usize,
) {
    api.rect(
        x,
        y,
        w as u16,
        h as u16,
        BORDER_COLOR,
    );

    api.rect(
        x + 3,
        y + 3,
        (w - 6) as u16,
        (h - 6) as u16,
        BUTTON_COLOR,
    );

    let label = match direction {
        0 => "UP",
        1 => "RIGHT",
        2 => "DOWN",
        _ => "LEFT",
    };

    let text_x = x + (w - label.len() as i16 * 8) / 2;
    let text_y = y + (h - 12) / 2;

    api.text(text_x, text_y, label, WHITE);
}

fn inside(
    x: i16,
    y: i16,
    bx: i16,
    by: i16,
    bw: i16,
    bh: i16,
) -> bool {
    x >= bx && x < bx + bw
        && y >= by && y < by + bh
}

fn draw_grid(
    api: &Api,
    offset_x: i16,
    offset_y: i16,
    cols: usize,
    rows: usize,
) {
    for x in 0..=cols {
        api.rect(
            offset_x + x as i16 * CELL,
            offset_y,
            1,
            (rows as i16 * CELL) as u16,
            GRID_COLOR,
        );
    }

    for y in 0..=rows {
        api.rect(
            offset_x,
            offset_y + y as i16 * CELL,
            (cols as i16 * CELL) as u16,
            1,
            GRID_COLOR,
        );
    }
}

fn main(api: &'static Api) -> i32 {
    let width = api.width as i16;
    let height = api.height as i16;

    // Размер игрового поля
    let cols = ((width - 2 * (SIDE_W + GAP)) / CELL)
        .max(5) as usize;

    let max_rows = MAX / cols;

    let rows = (((height - GRID_TOP - BOTTOM_AREA) / CELL)
        .max(5) as usize)
        .min(max_rows);

    if cols < 5 || rows < 5 || rows * cols > MAX {
        return 0;
    }

    let offset_x =
        (width - cols as i16 * CELL) / 2;

    let offset_y = GRID_TOP;

    // Координаты четырёх отдельных кнопок
    let up_x = (width - TOP_W) / 2;
    let up_y = 36;

    let down_x = up_x;
    let down_y = height - TOP_H - GAP;

    let side_y = offset_y
        + ((rows as i16 * CELL - SIDE_H) / 2).max(0);

    let left_x = GAP;
    let right_x = width - SIDE_W - GAP;

    let mut snake = [0u16; MAX];
    let mut length = 4usize;
    let mut direction = 1usize;
    let mut score = 0u32;
    let mut last_move = api.now();
    let mut game_over = false;

    let start_x = cols / 2;
    let start_y = rows / 2;

    for i in 0..length {
        snake[i] =
            (start_y * cols + start_x - i) as u16;
    }

    let mut food =
        (start_y.saturating_sub(3) * cols + start_x) as u16;

    loop {
        // Чёрный фон
        api.rect(
            0,
            0,
            api.width,
            api.height,
            BG_COLOR,
        );

        // Заголовок и счёт
        api.text(8, 5, "SNAKE", WHITE);

        let mut score_buf = [0u8; 10];
        let mut n = score;
        let mut pos = 10;

        if n == 0 {
            pos -= 1;
            score_buf[pos] = b'0';
        } else {
            while n > 0 && pos > 0 {
                pos -= 1;
                score_buf[pos] = b'0' + (n % 10) as u8;
                n /= 10;
            }
        }

        let score_text =
            core::str::from_utf8(&score_buf[pos..])
                .unwrap_or("0");

        api.text(8, 23, "SCORE:", WHITE);
        api.text(64, 23, score_text, HEAD_COLOR);

        // Серые клетки игрового поля
        draw_grid(
            api,
            offset_x,
            offset_y,
            cols,
            rows,
        );

        // Змейка
        for i in 0..length {
            let x = snake[i] as usize % cols;
            let y = snake[i] as usize / cols;

            api.rect(
                offset_x + x as i16 * CELL + 1,
                offset_y + y as i16 * CELL + 1,
                (CELL - 2) as u16,
                (CELL - 2) as u16,
                if i == 0 {
                    HEAD_COLOR
                } else {
                    SNAKE_COLOR
                },
            );
        }

        // Красная еда
        let fx = food as usize % cols;
        let fy = food as usize / cols;

        api.rect(
            offset_x + fx as i16 * CELL + 1,
            offset_y + fy as i16 * CELL + 1,
            (CELL - 2) as u16,
            (CELL - 2) as u16,
            FOOD_COLOR,
        );

        // Четыре независимые кнопки
        draw_button(
            api,
            up_x,
            up_y,
            TOP_W,
            TOP_H,
            0,
        );

        draw_button(
            api,
            left_x,
            side_y,
            SIDE_W,
            SIDE_H,
            3,
        );

        draw_button(
            api,
            right_x,
            side_y,
            SIDE_W,
            SIDE_H,
            1,
        );

        draw_button(
            api,
            down_x,
            down_y,
            TOP_W,
            TOP_H,
            2,
        );

        if game_over {
            api.text(
                8,
                height / 2,
                "GAME OVER",
                FOOD_COLOR,
            );

            api.text(
                8,
                height / 2 + 20,
                "CROWN TO RESTART",
                WHITE,
            );
        }

        api.show();

        // События и управление
        if let Some(event) = api.event(30) {
            match event.kind {
                BUTTON => return 0,

                CROWN => {
                    if game_over {
                        length = 4;
                        score = 0;
                        direction = 1;
                        game_over = false;

                        for i in 0..length {
                            snake[i] =
                                (start_y * cols + start_x - i) as u16;
                        }

                        food =
                            (start_y.saturating_sub(3) * cols
                                + start_x) as u16;

                        last_move = api.now();
                    } else {
                        let turn =
                            if event.delta > 0 { 1 } else { 3 };

                        direction = (direction + turn) % 4;
                    }
                }

                TOUCH_DOWN => {
                    if !game_over {
                        let x = event.x;
                        let y = event.y;

                        let new_direction =
                            if inside(
                                x, y,
                                up_x, up_y,
                                TOP_W, TOP_H,
                            ) {
                                Some(0)
                            } else if inside(
                                x, y,
                                right_x, side_y,
                                SIDE_W, SIDE_H,
                            ) {
                                Some(1)
                            } else if inside(
                                x, y,
                                down_x, down_y,
                                TOP_W, TOP_H,
                            ) {
                                Some(2)
                            } else if inside(
                                x, y,
                                left_x, side_y,
                                SIDE_W, SIDE_H,
                            ) {
                                Some(3)
                            } else {
                                None
                            };

                        if let Some(new_direction) = new_direction {
                            if (new_direction + 2) % 4 != direction {
                                direction = new_direction;
                            }
                        }
                    }
                }

                _ => {}
            }
        }

        // Движение змейки
        if !game_over
            && api.now().wrapping_sub(last_move) >= 150
        {
            last_move = api.now();

            let head = snake[0] as usize;
            let x = head % cols;
            let y = head / cols;

            let (nx, ny) = match direction {
                0 => (
                    x,
                    if y == 0 { rows - 1 } else { y - 1 },
                ),
                1 => ((x + 1) % cols, y),
                2 => (x, (y + 1) % rows),
                _ => (
                    if x == 0 { cols - 1 } else { x - 1 },
                    y,
                ),
            };

            let next = (ny * cols + nx) as u16;
            let eating = next == food;

            let check_length =
                if eating { length } else { length - 1 };

            let mut collision = false;

            for i in 0..check_length {
                if snake[i] == next {
                    collision = true;
                    break;
                }
            }

            if collision {
                game_over = true;
            } else {
                if eating && length < MAX {
                    length += 1;
                    score += 1;
                }

                for i in (1..length).rev() {
                    snake[i] = snake[i - 1];
                }

                snake[0] = next;

                if eating {
                    let mut candidate =
                        api.now() as usize % (cols * rows);

                    for _ in 0..cols * rows {
                        let occupied = snake[..length]
                            .iter()
                            .any(|&part| part as usize == candidate);

                        if !occupied {
                            food = candidate as u16;
                            break;
                        }

                        candidate =
                            (candidate + 1) % (cols * rows);
                    }
                }
            }
        }
    }
}
