/**
 * Copyright Soramitsu Co., Ltd. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0
 */

#include "ametsuchi/impl/peer_query_wsv.hpp"

#include <numeric>

#include "ametsuchi/wsv_query.hpp"

namespace iroha {
  namespace ametsuchi {

    PeerQueryWsv::PeerQueryWsv(std::shared_ptr<WsvQuery> wsv)
        : wsv_(std::move(wsv)) {}

    std::optional<std::vector<PeerQuery::wPeer>>
    PeerQueryWsv::getLedgerPeers() {
      return wsv_->getPeers();
    }

    std::optional<PeerQuery::wPeer> PeerQueryWsv::getLedgerPeerByPublicKey(
        shared_model::interface::types::PublicKeyHexStringView public_key)
        const {
      return wsv_->getPeerByPublicKey(public_key);
    }

  }  // namespace ametsuchi
}  // namespace iroha
