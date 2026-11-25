use std::ops::Range;

#[derive(Clone, Debug)]
pub struct Columns {
    pub icon: f32,
    pub status: f32,
    pub name: f32,
    pub ip: Option<f32>,
    pub ports: Option<f32>,
    pub protocol: Option<f32>,
    pub account: Option<f32>,
    pub actions: f32,
}
impl Columns {
    pub fn for_width(width: f32) -> Self {
        let ip = (width >= 440.0).then(|| (width * 0.23).clamp(110.0, 260.0));
        let ports = (width >= 660.0).then(|| (width * 0.10).clamp(80.0, 150.0));
        let protocol = (width >= 850.0).then_some(68.0);
        let account = (width >= 1100.0).then(|| (width * 0.12).clamp(110.0, 200.0));
        let (icon, status, actions) = if width < 240.0 {
            (20.0, 30.0, 44.0)
        } else {
            (28.0, 44.0, 64.0)
        };
        let fixed = icon + status + actions;
        let optional = [ip, ports, protocol, account];
        let gaps = (3 + optional.iter().filter(|w| w.is_some()).count()) as f32 * 8.0;
        let name =
            (width - 20.0 - fixed - gaps - optional.into_iter().flatten().sum::<f32>()).max(22.0);
        Self {
            icon,
            status,
            name,
            ip,
            ports,
            protocol,
            account,
            actions,
        }
    }
}

pub fn page_range(total: usize, size: usize, page: &mut usize) -> Range<usize> {
    let size = size.max(1);
    let pages = total.div_ceil(size).max(1);
    *page = (*page).min(pages - 1);
    let start = *page * size;
    start..(start + size).min(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn columns_fit_at_narrow_and_wide_widths() {
        for width in [
            160.0, 180.0, 300.0, 440.0, 659.0, 660.0, 849.0, 850.0, 1100.0, 2000.0,
        ] {
            let c = Columns::for_width(width);
            let optional = [c.ip, c.ports, c.protocol, c.account];
            let total = c.icon
                + c.status
                + c.name
                + c.actions
                + 20.0
                + (3 + optional.iter().filter(|v| v.is_some()).count()) as f32 * 8.0
                + optional.into_iter().flatten().sum::<f32>();
            assert!((total - width).abs() < 0.01, "{width}: {total}");
            assert!(c.name >= 22.0);
            assert!(c.actions >= 44.0);
        }
        assert!(Columns::for_width(300.0).ip.is_none());
        assert!(Columns::for_width(1200.0).account.is_some());
    }
    #[test]
    fn pagination_clamps_after_filtering_and_handles_partial_pages() {
        let mut page = 2;
        assert_eq!(page_range(23, 10, &mut page), 20..23);
        assert_eq!(page_range(8, 10, &mut page), 0..8);
        assert_eq!(page, 0);
        page = 10;
        assert_eq!(page_range(0, 10, &mut page), 0..0);
        assert_eq!(page, 0);
        page = 1;
        assert_eq!(page_range(23, 25, &mut page), 0..23);
    }
}
