# -*- mode: ruby -*-
# vi: set ft=ruby :

$script = <<-SCRIPT
apt-get update && apt-get install -y jq
SCRIPT

Vagrant.configure("2") do |config|
  config.vm.box = "utm/bookworm"
  config.vm.provision "shell",
      inline: $script
end
