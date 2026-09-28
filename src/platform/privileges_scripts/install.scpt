on run {daemon_file, agent_file, user}

  set sh1 to "echo " & quoted form of daemon_file & " > /Library/LaunchDaemons/ru.novodoc.remote_service.plist && chown root:wheel /Library/LaunchDaemons/ru.novodoc.remote_service.plist;"

  set sh2 to "echo " & quoted form of agent_file & " > /Library/LaunchAgents/ru.novodoc.remote_server.plist && chown root:wheel /Library/LaunchAgents/ru.novodoc.remote_server.plist;"

  set sh3 to "mkdir -p /var/root/Library/Preferences/ru.novodoc.remote; cp -rf " & quoted form of ("/Users/" & user & "/Library/Preferences/ru.novodoc.remote/Новодок remote.toml") & " /var/root/Library/Preferences/ru.novodoc.remote/;"

  set sh4 to "cp -rf " & quoted form of ("/Users/" & user & "/Library/Preferences/ru.novodoc.remote/Новодок remote2.toml") & " /var/root/Library/Preferences/ru.novodoc.remote/;"

  set sh5 to "launchctl load -w /Library/LaunchDaemons/ru.novodoc.remote_service.plist;"

  set sh to sh1 & sh2 & sh3 & sh4 & sh5

  do shell script sh with prompt "Новодок remote wants to install daemon and agent" with administrator privileges
end run
