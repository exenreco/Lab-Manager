import 'dart:io';

const deviceIcons = <String, String>{
  'Server': 'server',
  'Computer': 'monitor',
  'Database': 'database',
  'Cloud': 'cloud',
  'Gateway': 'router',
  'DellIdrac': 'brand-dell',
  'Proxmox': 'brand-proxmox',
  'Ubuntu': 'brand-ubuntu',
  'Debian': 'brand-debian',
  'Windows': 'brand-windows',
  'MacOs': 'brand-apple',
  'Linux': 'brand-linux',
  'Docker': 'brand-docker',
  'Nas': 'hard-drive',
  'Router': 'router',
  'Switch': 'network',
  'Plex': 'brand-plex',
  'PiHole': 'brand-pi-hole',
  'UniFi': 'brand-ubiquiti',
  'Unraid': 'brand-unraid',
  'TrueNas': 'brand-truenas',
  'Synology': 'brand-synology',
  'Qnap': 'brand-qnap',
  'RaspberryPi': 'brand-raspberry-pi',
  'HomeAssistant': 'brand-home-assistant',
  'Jellyfin': 'brand-jellyfin',
  'Emby': 'brand-emby',
  'Nextcloud': 'brand-nextcloud',
  'Portainer': 'brand-portainer',
  'Kubernetes': 'brand-kubernetes',
  'Grafana': 'brand-grafana',
  'Prometheus': 'brand-prometheus',
  'OpnSense': 'brand-opnsense',
  'PfSense': 'brand-pfsense',
  'Nginx': 'brand-nginx',
  'Caddy': 'brand-caddy',
  'Traefik': 'brand-traefik',
  'AdGuard': 'brand-adguard-home',
  'Tailscale': 'brand-tailscale',
  'WireGuard': 'brand-wireguard',
  'Vaultwarden': 'brand-vaultwarden',
  'Immich': 'brand-immich',
  'Sonarr': 'brand-sonarr',
  'Radarr': 'brand-radarr',
  'Sabnzbd': 'brand-sabnzbd',
  'Qbittorrent': 'brand-qbittorrent',
  'Frigate': 'brand-frigate',
  'EspHome': 'brand-esphome',
};
const operatingSystems = [
  'DebianUbuntu',
  'FedoraRhel',
  'Arch',
  'Windows',
  'MacOs',
  'Linux',
  'Unifi',
  'Idrac',
  'Other',
];
const osLabels = {
  'DebianUbuntu': 'Debian / Ubuntu',
  'FedoraRhel': 'Fedora / RHEL',
  'Arch': 'Arch Linux',
  'Windows': 'Windows',
  'MacOs': 'macOS',
  'Linux': 'Linux',
  'Unifi': 'UniFi OS',
  'Idrac': 'Dell iDRAC',
  'Other': 'Other / appliance',
};

class Device {
  final Map<String, dynamic> data;
  Device(Map<String, dynamic> data) : data = Map.of(data);
  factory Device.create(String ip) => Device({
    'ip': ip,
    'name': 'Device $ip',
    'username': 'root',
    'web_port': 80,
    'ssh_port': 22,
    'https': false,
    'icon': 'Server',
    'group': '',
    'tags': '',
    'notes': '',
    'links': '',
  });
  String get name => data['name'] as String;
  String get ip => data['ip'] as String;
  String get username => data['username'] as String;
  int get webPort => data['web_port'] as int;
  int get sshPort => data['ssh_port'] as int;
  String get icon => data['icon'] as String? ?? 'Server';
  String get group => data['group'] as String? ?? '';
  String get tags => data['tags'] as String? ?? '';
  String get notes => data['notes'] as String? ?? '';
  String get key => '$ip:$sshPort:$username';
  String get monitorKey =>
      '$ip|$webPort|$sshPort|$username|${data['https'] == true}';
  Uri get url => Uri(
    scheme: data['https'] == true ? 'https' : 'http',
    host: ip,
    port: webPort,
  );
  String get os =>
      data['os'] as String? ??
      switch (icon) {
        'Ubuntu' ||
        'Debian' ||
        'Proxmox' ||
        'RaspberryPi' ||
        'PiHole' => 'DebianUbuntu',
        'Windows' => 'Windows',
        'MacOs' => 'MacOs',
        'DellIdrac' => 'Idrac',
        'UniFi' => 'Unifi',
        'Linux' || 'Docker' => 'Linux',
        _ => 'Other',
      };
  String get updateCommand {
    var command = switch (os) {
      'DebianUbuntu' => 'sudo apt update && sudo apt upgrade',
      'FedoraRhel' => 'sudo dnf upgrade',
      'Arch' => 'sudo pacman -Syu',
      'MacOs' => 'softwareupdate --list',
      'Idrac' => 'racadm getsysinfo',
      _ => '',
    };
    if (username == 'root') command = command.replaceAll('sudo ', '');
    return command;
  }

  Device copy(Map<String, dynamic> changes) => Device({...data, ...changes});
  void validate() {
    if (InternetAddress.tryParse(ip) == null) {
      throw const FormatException('Enter a valid IPv4 or IPv6 address.');
    }
    if (name.trim().isEmpty || name.length > 80) {
      throw const FormatException('Device name must be 1–80 characters.');
    }
    if (!RegExp(r'^[A-Za-z0-9_][A-Za-z0-9_.-]{0,63}\$?$').hasMatch(username)) {
      throw const FormatException('Enter a valid SSH username.');
    }
    if (webPort < 1 || webPort > 65535 || sshPort < 1 || sshPort > 65535) {
      throw const FormatException('Ports must be between 1 and 65535.');
    }
    if (data['os'] != null && !operatingSystems.contains(data['os'])) {
      throw const FormatException('Unknown operating system.');
    }
    for (final field in ['group', 'tags', 'notes', 'links']) {
      if (data[field] != null &&
          (data[field] is! String || (data[field] as String).length > 16384)) {
        throw FormatException('Invalid $field.');
      }
    }
  }
}
