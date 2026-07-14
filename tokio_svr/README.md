``` shell
curl -vv -X POST -d "{\"k\":\"v\"}" -H "U_Version: 1abc" -H "Content-Type: application/json" "http://localhost:8088/user/abc.html?tid=123&bid=456"


curl -vv -X POST -d "{\"k\":\"v\"}" -H "U_Version: 1abc" "http://localhost:8088/user/abc.html?tid=123&bid=456"
```