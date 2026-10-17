use std::net::{IpAddr, Ipv4Addr};

fn ipv4_addresses() -> Vec<Ipv4Addr> {
    let mut addresses: Vec<_> = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|interface| interface.is_oper_up())
        .filter_map(|interface| match interface.ip() {
            IpAddr::V4(ip) if !ip.is_unspecified() => Some(ip),
            _ => None,
        })
        .collect();
    addresses.sort_unstable();
    addresses.dedup();
    addresses
}

pub(crate) fn lan_ipv4_address() -> Option<Ipv4Addr> {
    let interfaces = if_addrs::get_if_addrs().ok()?;
    #[cfg(windows)]
    {
        windows::primary_address(interfaces)
    }
    #[cfg(not(windows))]
    {
        interfaces
            .into_iter()
            .filter(|interface| interface.is_oper_up() && !interface.is_p2p())
            .find_map(|interface| match interface.ip() {
                IpAddr::V4(ip) if ip.is_private() => Some(ip),
                _ => None,
            })
    }
}

pub(crate) fn is_local_host(host: &str) -> bool {
    if matches!(host, "localhost" | "[::1]" | "::1") {
        return true;
    }
    host.parse::<Ipv4Addr>()
        .is_ok_and(|ip| ip.is_loopback() || ip.is_unspecified() || ipv4_addresses().contains(&ip))
}

#[cfg(windows)]
mod windows {
    use std::{collections::HashMap, net::IpAddr, net::Ipv4Addr, ptr};
    use windows_sys::Win32::{
        NetworkManagement::IpHelper::{
            FreeMibTable, GetIfEntry2, GetIpForwardTable2, GetIpInterfaceEntry, MIB_IF_ROW2,
            MIB_IPFORWARD_TABLE2, MIB_IPINTERFACE_ROW,
        },
        Networking::WinSock::AF_INET,
    };

    fn default_route_costs() -> HashMap<u32, u64> {
        let mut costs = HashMap::new();
        let mut table: *mut MIB_IPFORWARD_TABLE2 = ptr::null_mut();
        // Windows owns this variable-length table until FreeMibTable is called.
        unsafe {
            if GetIpForwardTable2(AF_INET, &mut table) != 0 || table.is_null() {
                return costs;
            }
            let rows =
                std::slice::from_raw_parts((*table).Table.as_ptr(), (*table).NumEntries as usize);
            for route in rows {
                if route.DestinationPrefix.PrefixLength != 0 || route.Loopback {
                    continue;
                }
                let mut interface = MIB_IPINTERFACE_ROW {
                    Family: AF_INET,
                    InterfaceIndex: route.InterfaceIndex,
                    ..Default::default()
                };
                let metric = if GetIpInterfaceEntry(&mut interface) == 0 {
                    interface.Metric
                } else {
                    0
                };
                let cost = u64::from(route.Metric) + u64::from(metric);
                costs
                    .entry(route.InterfaceIndex)
                    .and_modify(|previous: &mut u64| *previous = (*previous).min(cost))
                    .or_insert(cost);
            }
            FreeMibTable(table.cast());
        }
        costs
    }

    pub(super) fn primary_address(interfaces: Vec<if_addrs::Interface>) -> Option<Ipv4Addr> {
        let routes = default_route_costs();
        interfaces
            .into_iter()
            .filter(|interface| interface.is_oper_up() && !interface.is_p2p())
            .filter_map(|interface| {
                let IpAddr::V4(ip) = interface.ip() else {
                    return None;
                };
                if !ip.is_private() {
                    return None;
                }
                let index = interface.index?;
                let mut row = MIB_IF_ROW2 {
                    InterfaceIndex: index,
                    ..Default::default()
                };
                // InterfaceIndex identifies an enumerated adapter; Windows fills this C structure.
                if unsafe { GetIfEntry2(&mut row) } != 0 {
                    return None;
                }
                let hardware = row.InterfaceAndOperStatusFlags._bitfield & 1 != 0;
                let route = routes.get(&index).copied();
                let priority = match (hardware, route) {
                    (true, Some(_)) => 0,
                    (false, Some(_)) if matches!(row.Type, 6 | 71) => 1,
                    (true, None) => 2,
                    _ => return None,
                };
                Some(((priority, route.unwrap_or(u64::MAX), index, ip), ip))
            })
            .min_by_key(|(priority, _)| *priority)
            .map(|(_, ip)| ip)
    }
}
