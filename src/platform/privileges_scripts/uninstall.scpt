set sh1 to "launchctl unload -w /Library/LaunchDaemons/ru.novodoc.remote_service.plist;"
set sh2 to "/bin/rm /Library/LaunchDaemons/ru.novodoc.remote_service.plist;"
set sh3 to "/bin/rm /Library/LaunchAgents/ru.novodoc.remote_server.plist;"

set sh to sh1 & sh2 & sh3
do shell script sh with prompt "Новодок remote wants to unload daemon" with administrator privileges