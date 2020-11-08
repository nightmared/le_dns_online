#!/usr/bin/env sh

export ONLINE_API_KEY="<YOUR API KEY HERE>"

#Usage: dns_myapi_add _acme-challenge.www.domain.com  "XKrxpRBosdIKFzxW_CT3KLZNf6q0HG9i01zxXp5CPBs"
dns_online_rust_preloaded_add() {
    dnsapi/le_dns_online -a $ONLINE_API_KEY -n $1 update --new-value $2
}

#Usage: dns_myapi_rm _acme-challenge.www.domain.com  "XKrxpRBosdIKFzxW_CT3KLZNf6q0HG9i01zxXp5CPBs"
dns_online_rust_preloaded_rm() {
    dnsapi/le_dns_online -a $ONLINE_API_KEY -n $1 update --new-value "\"placeholder\""
}
